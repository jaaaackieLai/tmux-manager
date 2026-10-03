use crate::{
    Result,
    error::error,
    storage,
    tmux::{PaneId, SessionId, TmuxClient},
};
use futures_util::FutureExt;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};
use uuid::Uuid;

pub async fn start(tmux: &TmuxClient, config: &Path, session: &SessionId) -> Result<Child> {
    // Resolve the server explicitly before leaving the manager's environment.
    let socket = match &tmux.socket {
        Some(socket) => socket.clone(),
        None => PathBuf::from(
            tmux.checked(&[
                "display-message",
                "-p",
                "-t",
                session.as_str(),
                "#{socket_path}",
            ])
            .await?
            .trim(),
        ),
    };
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--socket")
        .arg(&socket)
        .arg("--config-file")
        .arg(config)
        .args(["prompt-dock-session", "--session", session.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // 獨立 process group：終端斷線送給前景群組的 SIGHUP 不會在清理前殺掉 supervisor。
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn()?;
    let mut output = BufReader::new(
        child
            .stdout
            .take()
            .ok_or_else(|| error("無法讀取底部列啟動結果"))?,
    );
    let mut ready = String::new();
    // Individual tmux operations already have a two-second timeout. A global
    // startup deadline incorrectly aborts valid sessions with many windows,
    // and SIGKILL prevents the supervisor from cleaning its owned resources.
    match output.read_line(&mut ready).await {
        Ok(_) if ready.trim() == "READY" => Ok(child),
        _ => {
            let output = child.wait_with_output().await?;
            Err(error(format!(
                "底部 prompt 列啟動失敗：{}",
                String::from_utf8_lossy(&output.stderr).trim()
            )))
        }
    }
}

struct SessionDock {
    tmux: TmuxClient,
    session: SessionId,
    owner: String,
    /// 啟動底部列 pane 的指令（不含 `--initial-pane` 的值），建立時已驗證為 UTF-8。
    dock_command: Vec<String>,
    previous_mouse: String,
}
impl SessionDock {
    async fn ensure_windows(&self) -> Result<()> {
        let windows = self
            .tmux
            .checked(&[
                "list-windows",
                "-t",
                self.session.as_str(),
                "-F",
                "#{window_id}",
            ])
            .await?;
        // 單一 window 失敗（例如剛被關閉）只跳過它，不影響其他底部列。
        for window in windows.lines() {
            let _ = self.ensure_window(window).await;
        }
        Ok(())
    }
    async fn ensure_window(&self, window: &str) -> Result<()> {
        let panes = self
            .tmux
            .checked(&[
                "list-panes",
                "-t",
                window,
                "-F",
                "#{pane_id}\t#{pane_active}\t#{@tmux_manager_dock}\t#{pane_height}\t#{window_height}",
            ])
            .await?;
        let rows: Vec<_> = panes
            .lines()
            .map(|s| s.split('\t').collect::<Vec<_>>())
            .collect();
        let has_work = rows.iter().any(|r| r.len() == 5 && r[2].is_empty());
        // 每列的 window_height 相同；太矮的 window 不放底部列。
        let tall = rows
            .first()
            .and_then(|r| r.get(4)?.parse::<u16>().ok())
            .is_some_and(|height| height >= MIN_WINDOW_HEIGHT);
        let bar_height = BAR_HEIGHT.to_string();
        if let Some(bar) = rows.iter().find(|r| r.len() == 5 && !r[2].is_empty()) {
            if bar[2] == self.owner && has_work {
                if tall && bar[3] != bar_height {
                    let _ = self
                        .tmux
                        .checked(&["resize-pane", "-t", bar[0], "-y", &bar_height])
                        .await;
                }
                return Ok(());
            }
            // 持有此 session 的鎖時，其他 owner 的底部列必定來自已結束的 supervisor；
            // 工作 pane 都已結束時也移除自己的底部列，讓 tmux 照常關閉 window。
            self.tmux.checked(&["kill-pane", "-t", bar[0]]).await?;
            if !has_work {
                return Ok(());
            }
        }
        let work = rows
            .iter()
            .find(|r| r.len() == 5 && r[1] == "1" && r[2].is_empty())
            .ok_or_else(|| error("找不到底部列上方的工作 pane"))?;
        if !tall {
            return Ok(());
        }
        let work = PaneId::parse(work[0])?;
        let mut args = vec![
            "split-window",
            "-v",
            "-f",
            "-d",
            "-l",
            &bar_height,
            "-t",
            work.as_str(),
            "-P",
            "-F",
            "#{pane_id}",
        ];
        args.extend(self.dock_command.iter().map(String::as_str));
        args.push(work.as_str());
        let bar = PaneId::parse(self.tmux.checked(&args).await?.trim())?;
        if let Err(e) = self
            .tmux
            .checked(&[
                "set-option",
                "-p",
                "-t",
                bar.as_str(),
                "@tmux_manager_dock",
                &self.owner,
            ])
            .await
        {
            let _ = self.tmux.checked(&["kill-pane", "-t", bar.as_str()]).await;
            return Err(e);
        }
        Ok(())
    }
    async fn cleanup(&self) {
        if let Ok(panes) = self
            .tmux
            .checked(&[
                "list-panes",
                "-s",
                "-t",
                self.session.as_str(),
                "-F",
                "#{pane_id}\t#{@tmux_manager_dock}",
            ])
            .await
        {
            let bars: Vec<_> = panes
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .filter(|(pane, owner)| *owner == self.owner && PaneId::parse(pane).is_ok())
                .map(|(pane, _)| pane)
                .collect();
            // 一次 tmux 呼叫刪除全部底部列，縮短持有鎖的時間；tmux 在某個指令失敗（例如
            // pane 已不存在）時會略過其後的指令，因此失敗時改逐一刪除。
            let chained: Vec<_> = bars
                .iter()
                .flat_map(|pane| [";", "kill-pane", "-t", pane])
                .skip(1)
                .collect();
            if !chained.is_empty() && self.tmux.checked(&chained).await.is_err() {
                for pane in bars {
                    let _ = self.tmux.checked(&["kill-pane", "-t", pane]).await;
                }
            }
        }
        let session = self.session.as_str();
        let mut args = vec!["set-option", "-u", "-t", session, SAVED_MOUSE];
        // 只在 mouse 仍是本工具設定的 on 時還原，保留使用者期間的手動變更。
        let options = session_options(&self.tmux, &self.session).await;
        if options
            .ok()
            .as_deref()
            .and_then(|o| option_value(o, "mouse"))
            == Some("on")
        {
            args.extend([";", "set-option", "-t", session]);
            if self.previous_mouse.is_empty() {
                args.extend(["-u", "mouse"]);
            } else {
                args.extend(["mouse", &self.previous_mouse]);
            }
        }
        let _ = self.tmux.checked(&args).await;
    }
}

/// Session option 保存 attach 前的 mouse 設定；supervisor 異常結束時，接手者仍能還原。
const SAVED_MOUSE: &str = "@tmux_manager_mouse";
/// 原本未設定 mouse 時的保存值（tmux option 無法區分空字串與未設定）。
const MOUSE_UNSET: &str = "unset";
/// 底部列高度，以及放得下底部列的最小 window 高度。
const BAR_HEIGHT: u16 = 4;
const MIN_WINDOW_HEIGHT: u16 = 7;
/// 前一個 supervisor 清理並放開鎖所需的最長時間；超過表示它仍在管理此 session。
const HANDOFF: Duration = Duration::from_secs(5);

async fn has_clients(tmux: &TmuxClient, session: &SessionId) -> Result<bool> {
    let clients = tmux
        .checked(&[
            "list-clients",
            "-t",
            session.as_str(),
            "-F",
            "#{client_name}",
        ])
        .await?;
    Ok(!clients.trim().is_empty())
}

/// Session 層級直接設定的 options（不含繼承的全域值），每行 `name value`。
async fn session_options(tmux: &TmuxClient, session: &SessionId) -> Result<String> {
    tmux.checked(&["show-options", "-t", session.as_str()])
        .await
}

fn option_value<'a>(options: &'a str, name: &str) -> Option<&'a str> {
    options
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix(' '))
        .map(|value| value.trim_matches('"'))
}

/// attach 前的 mouse 設定（空字串表示未設定），以及第一次接手時要寫入的保存值。
async fn original_mouse(
    tmux: &TmuxClient,
    session: &SessionId,
) -> Result<(String, Option<String>)> {
    let options = session_options(tmux, session).await?;
    Ok(match option_value(&options, SAVED_MOUSE) {
        Some(MOUSE_UNSET) => (String::new(), None),
        Some(saved) => (saved.into(), None),
        None => {
            let current = option_value(&options, "mouse")
                .unwrap_or_default()
                .to_string();
            let saved = if current.is_empty() {
                MOUSE_UNSET.into()
            } else {
                current.clone()
            };
            (current, Some(saved))
        }
    })
}

/// 每個 tmux session 一把 supervisor 鎖。socket 路徑先正規化，同一個 server 不論以哪種
/// 路徑寫法（symlink、相對路徑）連線都對應同一把鎖。鎖檔在暫存目錄，因此 TMPDIR 不同的
/// 兩個 manager 仍會各自取鎖。
pub fn lock_path(socket: &Path, session: &SessionId) -> PathBuf {
    let socket = std::fs::canonicalize(socket).unwrap_or_else(|_| socket.into());
    let key = storage::sha256_hex(format!("{}:{}", socket.display(), session.as_str()).as_bytes());
    std::env::temp_dir().join(format!("tmux-manager-dock-{key}.lock"))
}

fn announce_ready() -> Result<()> {
    println!("READY");
    std::io::stdout().flush()?;
    Ok(())
}

/// 底部列 pane 的啟動指令（最後再接上 `--initial-pane` 的值），路徑皆為絕對路徑。
pub fn dock_command(binary: &Path, socket: &Path, config: &Path) -> Result<Vec<String>> {
    Ok(vec![
        utf8(binary, "binary")?,
        "--socket".into(),
        utf8(socket, "socket")?,
        "--config-file".into(),
        utf8(config, "config")?,
        "prompt-dock".into(),
        "--initial-pane".into(),
    ])
}

/// 轉成絕對路徑：tmux 在工作 pane 的 cwd 啟動底部列，與 supervisor 的 cwd 不同。
fn utf8(path: &Path, what: &str) -> Result<String> {
    std::path::absolute(path)?
        .to_str()
        .map(String::from)
        .ok_or_else(|| error(format!("{what} 路徑不是 UTF-8")))
}

pub async fn run(tmux: TmuxClient, config: PathBuf, session: SessionId) -> Result<()> {
    let socket = tmux
        .socket
        .clone()
        .ok_or_else(|| error("底部列需要明確 socket"))?;
    let path = lock_path(&socket, &session);
    // 另一個 supervisor 持有鎖時，它可能仍在管理，也可能正在清理後退出。先回報 READY
    // 讓 attach 不必等待，再於交接期間內重試：取得鎖且仍有 client 才接手。
    let (_lock, announced) = match storage::try_lock(&path)? {
        Some(lock) => (lock, false),
        None => {
            announce_ready()?;
            let deadline = Instant::now() + HANDOFF;
            let lock = loop {
                if let Some(lock) = storage::try_lock(&path)? {
                    break lock;
                }
                if Instant::now() >= deadline {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            };
            if !has_clients(&tmux, &session).await.unwrap_or(false) {
                return Ok(());
            }
            (lock, true)
        }
    };
    let (previous_mouse, save_mouse) = original_mouse(&tmux, &session).await?;
    let dock = SessionDock {
        dock_command: dock_command(&std::env::current_exe()?, &socket, &config)?,
        tmux,
        session,
        owner: Uuid::new_v4().to_string(),
        previous_mouse,
    };
    // Register before creating resources. Signals received during startup are
    // buffered; finish the current bounded operations, then clean the owner.
    #[cfg(unix)]
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    #[cfg(unix)]
    let mut hangup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;
    let mut stop = Box::pin(async {
        #[cfg(unix)]
        tokio::select! { _ = interrupt.recv() => (), _ = terminate.recv() => (), _ = hangup.recv() => () }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    });
    let result: Result<()> = async {
        let session = dock.session.as_str();
        // 先保存原本的 mouse 再開啟，兩者同一次 tmux 呼叫完成。
        let mut args = match &save_mouse {
            Some(saved) => vec!["set-option", "-t", session, SAVED_MOUSE, saved, ";"],
            None => vec![],
        };
        args.extend(["set-option", "-t", session, "mouse", "on"]);
        dock.tmux.checked(&args).await?;
        dock.ensure_windows().await?;
        if (&mut stop).now_or_never().is_some() {
            return Err(error("底部列啟動已中止，已清理本次資源"));
        }
        // manager 只讀第一行；再次寫入已關閉的 pipe 會失敗。
        if !announced { announce_ready()?; }
        let started = Instant::now();
        let mut had_clients = false;
        let mut tick = tokio::time::interval(Duration::from_millis(500));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = &mut stop => break,
                _ = tick.tick() => {
                    // 偶發失敗等下一輪再試；只有 session 已不存在才結束。
                    let Ok(attached) = has_clients(&dock.tmux, &dock.session).await else {
                        if dock.tmux.checked(&["has-session", "-t", session]).await.is_err() { break; }
                        continue;
                    };
                    if attached { had_clients = true; let _ = dock.ensure_windows().await; }
                    else if had_clients || started.elapsed()>Duration::from_secs(10) {break;}
                }
            }
        }
        Ok(())
    }.await;
    dock.cleanup().await;
    result
}
