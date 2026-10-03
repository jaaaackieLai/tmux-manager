use super::*;
impl TmuxClient {
    pub async fn check_version(&self) -> Result<()> {
        let text = self.checked(&["-V"]).await?;
        let value = text.trim().strip_prefix("tmux ").unwrap_or("");
        let numeric: String = value
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let mut numbers = numeric.split('.');
        let major = numbers.next().and_then(|v| v.parse::<u32>().ok());
        let minor = numbers.next().and_then(|v| v.parse::<u32>().ok());
        if matches!((major, minor), (Some(major), Some(minor)) if major > 3 || (major == 3 && minor >= 3))
        {
            Ok(())
        } else {
            Err(error(
                "需 tmux 3.3+；舊版請升級。無 popup 時可用 prompts 管理模式。",
            ))
        }
    }
    pub async fn capabilities(&self, pane: &PaneId) -> Result<PaneCapabilities> {
        let output = self.checked(&["display-message", "-p", "-t", pane.as_str(), "#{pane_id}\t#{pane_dead}\t#{pane_input_off}\t#{pane_in_mode}\t#{pane_synchronized}\t#{bracket_paste_flag}\t#{pane_private_modes}\t#{session_name}/#{window_index}:#{window_name}.#{pane_index}"]).await?;
        let parts: Vec<_> = output.trim_end_matches('\n').split('\t').collect();
        if parts.len() != 8 || parts[0] != pane.as_str() {
            return Err(error("指定 pane 已消失或能力查詢失敗"));
        }
        if !parts[1..5].iter().all(|v| matches!(*v, "0" | "1")) {
            return Err(error("無法確認指定 pane 輸入狀態"));
        }
        let bracketed_paste = match parts[5] {
            "1" => Some(true),
            "0" => Some(false),
            _ if !parts[6].is_empty() => Some(parts[6].split(',').any(|s| s == "2004")),
            _ => None,
        };
        Ok(PaneCapabilities {
            description: parts[7].into(),
            bracketed_paste,
            dead: parts[1] == "1",
            input_off: parts[2] == "1",
            in_mode: parts[3] == "1",
            synchronized: parts[4] == "1",
        })
    }
    pub async fn supports_popup(&self) -> Result<bool> {
        Ok(self
            .checked(&["list-commands"])
            .await?
            .lines()
            .any(|s| s.starts_with("display-popup ")))
    }
}
