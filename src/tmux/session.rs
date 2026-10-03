use super::*;
const LIST_FORMAT: &str = "#{session_id}\t#{session_name}\t#{session_windows}\t#{session_created}";
fn section_marker() -> String {
    format!("\u{1}tmux-manager-section-{}\u{1}", uuid::Uuid::new_v4())
}
pub const PREVIEW_LINES: usize = 15;
fn parse_sessions(text: &str) -> Result<Vec<Session>> {
    text.lines()
        .map(|line| {
            let parts: Vec<_> = line.split('\t').collect();
            if parts.len() != 4 {
                return Err(error("無法解析 tmux session"));
            }
            Ok(Session {
                id: SessionId::parse(parts[0])?,
                name: parts[1].into(),
                windows: parts[2].parse().map_err(|_| error("無效 window 數量"))?,
                created: parts[3].parse().map_err(|_| error("無效建立時間"))?,
            })
        })
        .collect()
}
fn no_server(stderr: &str) -> bool {
    stderr.contains("no server running")
        || stderr.contains("No such file or directory")
        || stderr.contains("no sessions")
}
/// capture-pane 會以空白行補滿 pane 高度；先去掉尾端空白行再取最後 `lines` 行。
fn tail(output: &str, lines: usize) -> String {
    let all: Vec<_> = output.lines().collect();
    let end = all
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map_or(0, |i| i + 1);
    all[end.saturating_sub(lines)..end].join("\n")
}
fn capture_args<'a>(target: &'a str, start: &'a str) -> [&'a str; 6] {
    ["capture-pane", "-p", "-t", target, "-S", start]
}
/// 以 sentinel 行切開 `\;` 串接的多段輸出；最後一個 sentinel 之後的尾段也會保留。
fn sections(output: &str, section: &str) -> Vec<String> {
    // display-message -p 會把控制字元以八進位 escape 輸出。
    let printed_section = section.replace('\u{1}', "\\001");
    let mut parts = vec![String::new()];
    for line in output.split_inclusive('\n') {
        let marker = line.trim_end_matches('\n');
        if marker == section || marker == printed_section {
            parts.push(String::new());
        } else {
            parts.last_mut().unwrap().push_str(line);
        }
    }
    parts
}
impl TmuxClient {
    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        Ok(self.snapshot(None).await?.0)
    }
    /// 單次 tmux 呼叫取得 session 清單與（選填）該 session active pane 的 preview。
    pub async fn snapshot(
        &self,
        selected: Option<&SessionId>,
    ) -> Result<(Vec<Session>, Option<String>)> {
        let start = format!("-{}", PREVIEW_LINES - 1);
        let section = section_marker();
        let mut args = vec!["list-sessions", "-F", LIST_FORMAT];
        if let Some(id) = selected {
            args.extend([";", "display-message", "-p", &section, ";"]);
            args.extend(capture_args(id.as_str(), &start));
        }
        let output = self.run(&args, &[], false).await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts = sections(&stdout, &section);
        // list-sessions 成功才會印出 sentinel；之後失敗的只有 capture。
        if output.status != 0 && parts.len() < 2 {
            if no_server(&output.stderr) {
                return Ok((Vec::new(), None));
            }
            return Err(error(format!("tmux：{}", output.stderr.trim())));
        }
        if parts.len() > 2 {
            return Err(error("tmux 擷取分段數量不符"));
        }
        let preview = (output.status == 0)
            .then(|| parts.get(1).map(|text| tail(text, PREVIEW_LINES)))
            .flatten();
        Ok((parse_sessions(&parts[0])?, preview))
    }
    pub async fn list_panes(&self, session: &SessionId) -> Result<Vec<Pane>> {
        let text = self.checked(&["list-panes", "-s", "-t", session.as_str(), "-F", "#{pane_id}\t#{session_name}/#{window_index}:#{window_name}.#{pane_index} (#{pane_current_command})\t#{@tmux_manager_dock}"]).await?;
        let mut panes = Vec::new();
        for line in text.lines() {
            let (id, description) = line
                .split_once('\t')
                .ok_or_else(|| error("無法解析 tmux pane"))?;
            let (description, owner) = description.rsplit_once('\t').unwrap_or((description, ""));
            if owner.is_empty() {
                panes.push(Pane {
                    id: PaneId::parse(id)?,
                    description: description.into(),
                });
            }
        }
        Ok(panes)
    }
    /// session 作為 target 時 tmux 會解析為其 active pane。
    pub async fn preview(&self, session: &SessionId) -> Result<String> {
        self.capture_target(session.as_str(), PREVIEW_LINES).await
    }
    pub async fn rename(&self, session: &SessionId, name: &str) -> Result<()> {
        self.checked(&["rename-session", "-t", session.as_str(), name])
            .await
            .map(|_| ())
    }
    pub async fn kill(&self, session: &SessionId) -> Result<()> {
        self.checked(&["kill-session", "-t", session.as_str()])
            .await
            .map(|_| ())
    }
    async fn capture_target(&self, target: &str, lines: usize) -> Result<String> {
        let start = format!("-{}", lines.saturating_sub(1));
        let args = capture_args(target, &start);
        let output = self.checked(&args).await?;
        Ok(tail(&output, lines))
    }
    /// 單次 tmux 呼叫依序擷取多個 pane，每個 pane 取最後 `lines` 行。
    pub async fn capture_many(&self, panes: &[PaneId], lines: usize) -> Result<Vec<String>> {
        if panes.is_empty() {
            return Ok(Vec::new());
        }
        let start = format!("-{}", lines.saturating_sub(1));
        let mut args = Vec::new();
        let section = section_marker();
        for pane in panes {
            if !args.is_empty() {
                args.push(";");
            }
            args.extend(capture_args(pane.as_str(), &start));
            args.extend([";", "display-message", "-p", &section]);
        }
        let parts = sections(&self.checked(&args).await?, &section);
        if parts.len() != panes.len() + 1 {
            return Err(error("tmux 擷取輸出不完整"));
        }
        Ok(parts[..panes.len()]
            .iter()
            .map(|p| tail(p, lines))
            .collect())
    }
    pub async fn create(&self, name: &str, directory: &str, command: &str) -> Result<SessionId> {
        let mut args = vec!["new-session", "-d", "-P", "-F", "#{session_id}", "-s", name];
        if !directory.is_empty() {
            args.extend(["-c", directory]);
        }
        let id = SessionId::parse(self.checked(&args).await?.trim())?;
        if !command.is_empty() {
            let pane = self
                .list_panes(&id)
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| error("新 session 沒有 pane"))?;
            self.checked(&["send-keys", "-l", "-t", pane.id.as_str(), command])
                .await?;
            self.checked(&["send-keys", "-t", pane.id.as_str(), "Enter"])
                .await?;
        }
        Ok(id)
    }
    pub async fn attach(&self, session: &SessionId, inside_tmux: bool) -> Result<()> {
        let action = if inside_tmux {
            "switch-client"
        } else {
            "attach-session"
        };
        let result = self
            .run(&[action, "-t", session.as_str()], &[], !inside_tmux)
            .await?;
        if result.status == 0 {
            Ok(())
        } else {
            Err(error(format!("tmux attach 失敗：{}", result.stderr)))
        }
    }
}
