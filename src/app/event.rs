use crate::{
    ai::AiSummary,
    tmux::{PanePreview, Session, SessionId},
};
pub enum AppEvent {
    Error(String),
    Refresh {
        sessions: Vec<Session>,
        preview: Option<(SessionId, Vec<PanePreview>)>,
    },
    AiResult {
        session_id: SessionId,
        generation: u64,
        result: std::result::Result<AiSummary, String>,
    },
}
/// 在背景執行並送出結果事件。工作 panic 時改送錯誤事件，等待結果的狀態
/// （例如 manager 的 io_busy）才不會永遠卡住。
pub fn spawn_event(
    sender: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    work: impl Future<Output = AppEvent> + Send + 'static,
) {
    tokio::spawn(async move {
        let event = tokio::spawn(work)
            .await
            .unwrap_or_else(|e| AppEvent::Error(format!("背景更新失敗：{e}")));
        let _ = sender.send(event);
    });
}
