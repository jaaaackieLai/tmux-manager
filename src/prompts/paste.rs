use super::model::normalize_body;
use crate::{
    Result,
    error::error,
    tmux::{PaneCapabilities, PaneId, TmuxClient},
};
use std::path::PathBuf;
#[derive(Clone, Debug)]
pub struct PasteTarget {
    pub socket: Option<PathBuf>,
    pub pane_id: PaneId,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PasteOutcome {
    Delivered { warning: Option<String> },
    BufferedOnly { buffer: String, reason: String },
}
#[derive(Clone)]
pub struct PasteService {
    tmux: TmuxClient,
}
fn ready(capabilities: &PaneCapabilities) -> Result<()> {
    if capabilities.dead {
        return Err(error("指定 pane 已結束"));
    }
    if capabilities.input_off {
        return Err(error("指定 pane 已關閉輸入"));
    }
    if capabilities.in_mode {
        return Err(error("指定 pane 處於 copy mode 或其他模式，請先退出"));
    }
    Ok(())
}
impl PasteService {
    pub fn new(tmux: TmuxClient) -> Self {
        Self { tmux }
    }
    pub async fn paste(&self, target: &PasteTarget, body: &str) -> Result<PasteOutcome> {
        if target.socket != self.tmux.socket {
            return Err(error("目標 socket 不符，停止貼上"));
        }
        let body = normalize_body(body)?;
        let capabilities = self.tmux.capabilities(&target.pane_id).await?;
        ready(&capabilities)?;
        let buffer = format!("tmux-manager-{}", uuid::Uuid::new_v4());
        self.tmux
            .checked_with_stdin(&["load-buffer", "-b", &buffer, "-"], body.as_bytes())
            .await?;
        let multiline = body.contains('\n');
        if multiline && capabilities.bracketed_paste != Some(true) {
            return Ok(PasteOutcome::BufferedOnly { buffer, reason: "尚未貼上：無法確認 mode 2004 已開啟；請升級可查詢 bracketed paste 的 tmux，或在目標程式開啟此模式".into() });
        }
        let deliver = async {
            let current = self.tmux.capabilities(&target.pane_id).await?;
            ready(&current)?;
            if multiline && current.bracketed_paste != Some(true) {
                return Ok(PasteOutcome::BufferedOnly {
                    buffer: buffer.clone(),
                    reason: "尚未貼上：目標的 bracketed paste 狀態已改變".into(),
                });
            }
            self.tmux
                .checked(&[
                    "paste-buffer",
                    "-p",
                    "-r",
                    "-b",
                    &buffer,
                    "-t",
                    target.pane_id.as_str(),
                ])
                .await?;
            let warning = self
                .tmux
                .checked(&["delete-buffer", "-b", &buffer])
                .await
                .err()
                .map(|e| format!("已交付 pane；buffer {buffer} 清理失敗：{e}；勿重貼"));
            Ok::<_, crate::error::Error>(PasteOutcome::Delivered { warning })
        };
        deliver.await.map_err(|e| {
            error(format!(
                "{e}；本次 buffer 保留為 {buffer}。不會自動重試或改貼其他 pane。"
            ))
        })
    }
}
