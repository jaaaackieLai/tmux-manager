use super::{PaneId, TmuxClient};
use crate::{Result, error::error};

impl TmuxClient {
    pub async fn dock_target(&self, bar: &PaneId) -> Result<PaneId> {
        let text = self
            .checked(&[
                "list-panes",
                "-t",
                bar.as_str(),
                "-F",
                "#{pane_id}\t#{pane_active}\t#{pane_last}\t#{@tmux_manager_dock}",
            ])
            .await?;
        let panes: Vec<_> = text
            .lines()
            .filter_map(|line| {
                let parts: Vec<_> = line.split('\t').collect();
                (parts.len() == 4).then_some(parts)
            })
            .collect();
        for flag in [1, 2] {
            if let Some(pane) = panes
                .iter()
                .find(|parts| parts[0] != bar.as_str() && parts[3].is_empty() && parts[flag] == "1")
            {
                return PaneId::parse(pane[0]);
            }
        }
        Err(error("找不到上方最近使用的工作 pane；請先點選工作區"))
    }
}
