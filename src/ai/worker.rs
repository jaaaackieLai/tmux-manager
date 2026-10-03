use super::AiClient;
use crate::{
    app::event::AppEvent,
    tmux::{Session, TmuxClient},
};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Semaphore, mpsc},
    task::JoinHandle,
};
pub struct AiService {
    client: AiClient,
    tmux: TmuxClient,
    sender: mpsc::UnboundedSender<AppEvent>,
    jobs: Vec<JoinHandle<()>>,
    semaphore: Arc<Semaphore>,
}
impl AiService {
    pub fn new(
        client: AiClient,
        tmux: TmuxClient,
        sender: mpsc::UnboundedSender<AppEvent>,
    ) -> Self {
        Self {
            client,
            tmux,
            sender,
            jobs: Vec::new(),
            semaphore: Arc::new(Semaphore::new(4)),
        }
    }
    pub fn refresh(&mut self, sessions: Vec<Session>, generation: u64) {
        for job in self.jobs.drain(..) {
            job.abort();
        }
        if !self.client.enabled() {
            return;
        }
        for session in sessions {
            let (client, tmux, sender, semaphore) = (
                self.client.clone(),
                self.tmux.clone(),
                self.sender.clone(),
                self.semaphore.clone(),
            );
            self.jobs.push(tokio::spawn(async move {
                let Ok(_permit) = semaphore.acquire_owned().await else {
                    return;
                };
                let id = session.id;
                let target = id.clone();
                let result = tokio::time::timeout(Duration::from_secs(15), async move {
                    let panes = tmux.list_panes(&target).await?;
                    let budget = 80usize;
                    let per_pane = budget
                        .saturating_sub(panes.len())
                        .checked_div(panes.len())
                        .unwrap_or(0);
                    let ids: Vec<_> = panes.iter().map(|p| p.id.clone()).collect();
                    let captures = if per_pane > 0 {
                        tmux.capture_many(&ids, per_pane).await?
                    } else {
                        Vec::new()
                    };
                    let mut lines = Vec::new();
                    for (index, pane) in panes.iter().enumerate() {
                        if lines.len() >= budget {
                            break;
                        }
                        lines.push(format!("[{} {}]", pane.id.as_str(), pane.description));
                        if let Some(text) = captures.get(index) {
                            lines.extend(text.lines().map(String::from));
                        }
                    }
                    lines.truncate(budget);
                    let capture = lines.join("\n");
                    client
                        .summarize(&capture)
                        .await?
                        .ok_or_else(|| crate::error::error("AI 未啟用"))
                })
                .await;
                let result = match result {
                    Ok(value) => value.map_err(|e| e.to_string()),
                    Err(_) => Err("AI 摘要超時（15 秒）".into()),
                };
                let _ = sender.send(AppEvent::AiResult {
                    session_id: id,
                    generation,
                    result,
                });
            }));
        }
    }
}
impl Drop for AiService {
    fn drop(&mut self) {
        for job in &self.jobs {
            job.abort();
        }
    }
}
