use super::{
    PromptStore,
    editor::{EditorOutcome, PromptEditor},
    paste::{PasteOutcome, PasteService, PasteTarget},
    ui::{PromptAction, PromptView},
};
use crate::{
    Result,
    tmux::TmuxClient,
    ui::terminal::{Input, UiTerminal},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;
use uuid::Uuid;
enum Confirmation {
    Delete(Uuid),
    Discard,
}
pub async fn run(
    terminal: &mut UiTerminal,
    input: &mut Input,
    store: PromptStore,
    tmux: TmuxClient,
    target: Option<PasteTarget>,
) -> Result<()> {
    let snapshot = store.load()?;
    let mut status = String::new();
    let description = if let Some(target) = &target {
        match tmux.capabilities(&target.pane_id).await {
            Ok(caps) => Some(format!(
                "{} · {}",
                target.pane_id.as_str(),
                caps.description
            )),
            Err(e) => {
                status = e.to_string();
                Some(format!("{}（不可用）", target.pane_id.as_str()))
            }
        }
    } else {
        status =
            "未指定 target；可新增/編輯，貼上需 tmux pane ID。用 bindings --print 設定 popup。"
                .into();
        None
    };
    let mut view = PromptView::new(snapshot, description);
    view.status = status;
    let service = PasteService::new(tmux);
    let mut confirmation = None;
    let mut dirty = true;
    let mut reload = tokio::time::interval(Duration::from_millis(500));
    // 編輯期間暫停重讀；回到列表時不補跑累積的 tick。
    reload.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if dirty {
            terminal.draw(|f| {
                view.render(f);
            })?;
            dirty = false;
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => return Ok(()),
            _ = reload.tick(), if view.editor.is_none() && confirmation.is_none() => {
                dirty |= store.reload_into(&mut view.snapshot, &mut view.reload_error);
            },
            event = input.next() => {
                let event = event?;
                if matches!(&event,Event::Key(key) if key.kind != KeyEventKind::Press) { continue; }
                dirty = true;
                if let Some(confirm) = confirmation.take() {
                    match event {
                        Event::Key(key) if key.code == KeyCode::Char('y') => {
                            match confirm { Confirmation::Discard => { view.editor = None; view.status.clear(); }, Confirmation::Delete(id) => { if let Err(e) = view.delete(&store,id) { view.status = e.to_string(); } } }
                        }
                        Event::Key(_) => view.status = "已取消".into(),
                        _ => confirmation = Some(confirm),
                    }
                    continue;
                }
                if let Event::Key(key) = &event { if matches!(key.code, KeyCode::PageDown | KeyCode::PageUp) { view.handle_key(*key); continue; } }
                if view.editor.is_some() {
                    if matches!(&event,Event::Key(key) if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r')) {
                        match store.load() { Ok(snapshot) => { view.snapshot = snapshot; view.status = "已載入外部資料，草稿仍保留；檢查後 Ctrl-S 儲存".into(); }, Err(e) => view.status = e.to_string() }
                        continue;
                    }
                    match view.editor.as_mut().unwrap().handle(event) {
                        Ok(EditorOutcome::Save(slot)) => { if let Err(e) = view.save(&store,slot) { view.status = format!("{e}；Ctrl-R 重讀並保留草稿"); } },
                        Ok(EditorOutcome::Cancel) => { view.editor = None; view.status.clear(); },
                        Ok(EditorOutcome::ConfirmDiscard) => { confirmation = Some(Confirmation::Discard); view.status = "放棄未儲存草稿？[y/N]".into(); },
                        Err(e) => view.status = e.to_string(),
                        _ => (),
                    }
                    continue;
                }
                let action = match event { Event::Key(key) => view.handle_key(key), Event::Mouse(mouse) => view.handle_mouse(mouse), _ => None };
                match action {
                    Some(PromptAction::Cancel) => return Ok(()),
                    Some(PromptAction::Create) => { let order = view.snapshot.document.slots.iter().map(|s| s.order).max().map(|o| o.saturating_add(1)).unwrap_or(0); view.editor = Some(PromptEditor::create(order)); },
                    Some(PromptAction::Edit(id)) => { if let Some(slot) = view.snapshot.document.slots.iter().find(|s| s.id == id) { view.editor = Some(PromptEditor::edit(slot)); } },
                    Some(PromptAction::Delete(id)) => { confirmation = Some(Confirmation::Delete(id)); view.status = "確定刪除此 prompt？[y/N]".into(); },
                    Some(PromptAction::Move(id,delta)) => { if let Err(e) = view.move_slot(&store,id,delta) { view.status = e.to_string(); } },
                    Some(PromptAction::Reload) => match store.load() { Ok(snapshot) => { view.snapshot = snapshot; view.status = "已重新載入".into(); }, Err(e) => view.status = e.to_string() },
                    Some(PromptAction::Paste(id)) => {
                        let Some(target) = target.clone() else { view.status = "未指定 target，禁止貼上".into(); continue; };
                        // 以檔案目前內容為準：絕不貼上已刪除或被外部修改的舊內容。
                        let slot = match store.current_slot(&mut view.snapshot, id) {
                            Ok(slot) => slot,
                            Err(e) => { view.status = e.to_string(); continue; },
                        };
                        match after_paste(service.paste(&target,&slot.body).await) {
                            None => return Ok(()),
                            Some(status) => view.status = status,
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}

/// popup 貼上後：`None` 表示完成並關閉；否則留在畫面顯示狀態（含已貼上但需注意的警告）。
pub fn after_paste(result: Result<PasteOutcome>) -> Option<String> {
    match result {
        Ok(PasteOutcome::Delivered { warning }) => warning,
        Ok(PasteOutcome::BufferedOnly { buffer, reason }) => {
            Some(format!("{reason} · buffer: {buffer}"))
        }
        Err(e) => Some(e.to_string()),
    }
}
