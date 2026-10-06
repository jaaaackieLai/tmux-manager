use super::{
    AppState, ManagerAction, Screen, actions,
    event::{self, AppEvent},
};
use crate::{
    Result,
    ai::{AiClient, AiService},
    config::Config,
    prompts::{PromptStore, paste::PasteTarget, runtime as prompts},
    tmux::{PaneLayout, SessionId, TmuxClient},
    ui::{
        form, manager,
        terminal::{self, Input, TerminalGuard},
    },
};
use crossterm::event::{Event, KeyEventKind, MouseEventKind};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;
pub async fn run(
    tmux: TmuxClient,
    config: Config,
    key: Option<String>,
    inside_tmux: bool,
) -> Result<()> {
    let sessions = tmux.list_sessions().await?;
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let mut ai = AiService::new(
        AiClient::new(
            key,
            &config.ai_model,
            "https://api.anthropic.com/v1/messages",
        )?,
        tmux.clone(),
        sender.clone(),
    );
    let mut app = AppState {
        status: config.notice.clone().unwrap_or_default(),
        ..AppState::default()
    };
    app.replace_sessions(sessions);
    app.generation = 1;
    ai.refresh(app.sessions.clone(), app.generation);
    let mut guard = TerminalGuard::enter()?;
    let mut terminal = terminal::terminal()?;
    let mut poll = tokio::time::interval(Duration::from_secs_f64(config.poll_interval.max(0.02)));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut input = Input::new();
    let mut io_busy = false;
    let mut dirty = true;
    let mut hits = crate::ui::manager_mouse::ManagerHitMap::default();
    let pane_cache = Arc::new(Mutex::new(Vec::<PaneLayout>::new()));
    loop {
        if dirty {
            terminal.draw(|f| {
                hits = manager::render(f, &app);
            })?;
            dirty = false;
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => return Ok(()),
            Some(message) = receiver.recv() => match message {
                AppEvent::Refresh { sessions,preview } => {
                    io_busy = false; dirty |= app.replace_sessions(sessions); dirty |= app.refresh_succeeded();
                    if let Some((id,text)) = preview { if app.selected_id() == Some(&id) && app.preview != text { app.preview = text; dirty = true; } }
                },
                AppEvent::AiResult { session_id,generation,result } => dirty |= app.apply_ai(session_id,generation,result),
                AppEvent::Error(message) => { io_busy = false; dirty |= app.show_refresh_error(message); },
            },
            _ = poll.tick(), if !io_busy => {
                io_busy = true;
                // 操作選單只覆蓋部分列表；背景 Preview 仍持續更新。
                let selected = app.selected_id().cloned();
                let client = tmux.clone();
                let cache = pane_cache.clone();
                event::spawn_event(sender.clone(), async move {
                    // 暫時：上一輪的 pane 清單作為本輪擷取依據，Preview 只顯示 active pane。
                    let panes = cache.lock().unwrap().clone();
                    let result = client.snapshot(selected.as_ref().map(|id| (id,panes.as_slice()))).await.map(|snapshot| {
                        let text = snapshot.preview.and_then(|panes| {
                            *cache.lock().unwrap() = panes.iter().map(|p| p.layout.clone()).collect();
                            panes.into_iter().find(|p| p.layout.active).map(|p| p.text)
                        });
                        AppEvent::Refresh { sessions: snapshot.sessions,preview: selected.zip(text) }
                    });
                    result.unwrap_or_else(|e| AppEvent::Error(e.to_string()))
                });
            },
            event = input.next() => {
                let event = event?;
                let old_id = app.selected_id().cloned();
                let action = match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if app.handle_layout_key(key, &hits) { ManagerAction::None } else { app.handle_key(key) }
                    },
                    // 純移動不改狀態；其餘交給 ratatui diff，未變的 cell 不會重送。
                    Event::Mouse(mouse) if mouse.kind == MouseEventKind::Moved => continue,
                    Event::Mouse(mouse) => app.handle_mouse(mouse, &hits),
                    Event::Resize(_,_) => { app.dragging_split = false; dirty = true; continue; },
                    _ => continue,
                }; dirty = true;
                if old_id.as_ref() != app.selected_id() { app.preview.clear(); }
                // 只有離開過 manager 畫面（attach/表單/prompts）才需整頁重畫。
                let left_screen = !matches!(action, ManagerAction::None | ManagerAction::Refresh);
                // 動作失敗（含表單 Ctrl-C、attach 失敗）只顯示在狀態列，不結束 manager。
                let outcome: Result<bool> = async {
                    match action {
                        ManagerAction::Quit => return Ok(true),
                        ManagerAction::Refresh => { app.generation += 1; app.ai.clear(); ai.refresh(app.sessions.clone(),app.generation); },
                        ManagerAction::New | ManagerAction::Attach => {
                            let id = if action == ManagerAction::New { actions::new_session(&mut terminal,&mut input,&tmux,&config).await? } else { app.selected_id().cloned() };
                            if let Some(id) = id {
                                let warning = attach(&mut guard,&mut input,&tmux,&config.path,&id,inside_tmux).await?;
                                if inside_tmux { return Ok(true); }
                                app.screen = Screen::List;
                                if let Some(warning) = warning { app.status = warning; }
                            }
                        },
                        ManagerAction::Rename => if let Some(id) = app.selected_id().cloned() {
                            let suggestion = app.ai.get(&id).and_then(|v| v.as_ref().ok()).map(|s| s.name.as_str()).unwrap_or("");
                            if let Some(name) = form::text(&mut terminal,&mut input,"改名（Enter 接受 AI 建議，Ctrl-U 清空）",suggestion).await?.filter(|s| !s.trim().is_empty()) {
                                tmux.rename(&id,&name).await?; app.screen = Screen::List;
                            }
                        },
                        ManagerAction::Kill => if let Some(id) = app.selected_id().cloned() {
                            if form::confirm(&mut terminal,&mut input,"確定結束此 session？").await? {
                                tmux.kill(&id).await?; app.screen = Screen::List;
                            }
                        },
                        ManagerAction::Prompts => {
                            // 列表頁只管理 prompts；操作選單先選貼上的 pane，取消就不開啟。
                            let target = if app.screen == Screen::List { None } else {
                                let Some(id) = app.selected_id().cloned() else { return Ok(false); };
                                let panes = tmux.list_panes(&id).await?;
                                let Some(pane) = actions::pane_picker(&mut terminal,&mut input,panes).await? else { return Ok(false); };
                                Some(PasteTarget { socket:tmux.socket.clone(),pane_id:pane.id })
                            };
                            prompts::run(&mut terminal,&mut input,PromptStore::new(config.prompts_path()),tmux.clone(),target).await?;
                        },
                        ManagerAction::None => (),
                    }
                    Ok(false)
                }.await;
                match outcome { Ok(true) => return Ok(()), Ok(false) => (), Err(e) => app.status = e.to_string() }
                if left_screen { terminal.clear()?; }
            }
        }
    }
}
async fn attach(
    guard: &mut TerminalGuard,
    input: &mut Input,
    tmux: &TmuxClient,
    config_path: &std::path::Path,
    id: &SessionId,
    inside_tmux: bool,
) -> Result<Option<String>> {
    input.suspend();
    guard.suspend()?;
    let (supervisor, result) = actions::attach_with_dock(
        crate::prompts::dock_session::start(tmux, config_path, id),
        tmux.attach(id, inside_tmux),
    )
    .await;
    if let Some(mut supervisor) = supervisor {
        // The independent supervisor cleans itself after the last client detaches.
        // Reap when it exits, without delaying the manager's return.
        tokio::spawn(async move {
            let _ = supervisor.wait().await;
        });
    }
    guard.resume()?;
    result
}
