use super::{
    PromptSnapshot, PromptStore,
    dock_ui::DockView,
    paste::{PasteOutcome, PasteService, PasteTarget},
};
use crate::{
    Result,
    error::error,
    tmux::{PaneId, TmuxClient},
    ui::terminal::{Input, UiTerminal},
};
use crossterm::event::{Event, MouseButton, MouseEventKind};
use std::time::Duration;
use uuid::Uuid;

pub async fn run(
    terminal: &mut UiTerminal,
    input: &mut Input,
    store: PromptStore,
    tmux: TmuxClient,
    initial_pane: PaneId,
) -> Result<()> {
    let bar =
        PaneId::parse(&std::env::var("TMUX_PANE").map_err(|_| error("底部列需要 tmux pane"))?)?;
    if bar == initial_pane {
        return Err(error("底部列不能以自己為工作 pane"));
    }
    let service = PasteService::new(tmux.clone());
    let mut view = DockView::default();
    let mut snapshot = store.load()?;
    let mut status = " 點選 prompt 填入工作區，再由你編輯與送出".to_string();
    let mut reload_error = None;
    let mut dirty = true;
    let mut reload = tokio::time::interval(Duration::from_millis(500));
    reload.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut ctrl_c = std::pin::pin!(tokio::signal::ctrl_c());
    loop {
        if dirty {
            let slots: Vec<_> = snapshot.document.search("").into_iter().cloned().collect();
            // 讀取錯誤優先顯示，恢復後回到原本的狀態訊息。
            terminal
                .draw(|f| view.render(f, &slots, reload_error.as_deref().unwrap_or(&status)))?;
            dirty = false;
        }
        tokio::select! {
            _ = &mut ctrl_c => return Ok(()),
            _ = reload.tick() => dirty |= store.reload_into(&mut snapshot, &mut reload_error),
            event = input.next() => match event? {
                Event::Resize(_,_) => dirty = true,
                Event::Mouse(mouse) => {
                    let action = view.handle_mouse(mouse);
                    // 捲動只翻頁；tmux 的滾輪不會切換 active pane，不需交回焦點。
                    if matches!(mouse.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) {
                        dirty = true;
                    } else if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                        dirty = true;
                        match tmux.dock_target(&bar).await {
                            Ok(target) => {
                                if let Some(id) = action {
                                    status = paste_clicked(&store, &service, &tmux, &mut snapshot, &target, id).await;
                                }
                                if let Err(e) = refocus(&tmux, &target).await { status = e.to_string(); }
                            },
                            Err(e) => status = e.to_string(),
                        }
                    }
                },
                // Keyboard never navigates or triggers prompts. Return focus if selected via tmux navigation.
                Event::Key(_) => {
                    if let Ok(target) = tmux.dock_target(&bar).await {
                        let _ = refocus(&tmux, &target).await;
                    }
                },
                _ => (),
            }
        }
    }
}

/// 貼上點選的 prompt（以檔案目前內容為準）；回傳狀態列文字。
async fn paste_clicked(
    store: &PromptStore,
    service: &PasteService,
    tmux: &TmuxClient,
    snapshot: &mut PromptSnapshot,
    target: &PaneId,
    id: Uuid,
) -> String {
    let slot = match store.current_slot(snapshot, id) {
        Ok(slot) => slot,
        Err(e) => return e.to_string(),
    };
    let target = PasteTarget {
        socket: tmux.socket.clone(),
        pane_id: target.clone(),
    };
    match service.paste(&target, &slot.body).await {
        Ok(PasteOutcome::Delivered { warning }) => {
            warning.unwrap_or_else(|| format!(" 已填入「{}」；由你編輯與送出", slot.title))
        }
        Ok(PasteOutcome::BufferedOnly { buffer, .. }) => format!("多行尚未貼上 · {buffer}"),
        Err(e) => e.to_string(),
    }
}

async fn refocus(tmux: &TmuxClient, target: &PaneId) -> Result<()> {
    tmux.checked(&["select-pane", "-t", target.as_str()])
        .await
        .map(drop)
}
