use std::time::Duration;
use tmux_manager::{
    ai::{AiClient, AiService},
    app::event::AppEvent,
    tmux::{PaneId, PanePreview, SessionId, TmuxClient},
};

/// Workspace 第一個 pane 印出的內容。
const WORKSPACE_TEXT: &str =
    "WORKSPACE-CAPTURE-MARKER\n\\001tmux-manager-section\\001\nWORKSPACE-AFTER-MARKER";

fn texts(preview: &[PanePreview]) -> Vec<&str> {
    preview.iter().map(|p| p.text.as_str()).collect()
}

struct Workspace {
    _dir: tempfile::TempDir,
    client: TmuxClient,
    session: SessionId,
}

impl Workspace {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let client = TmuxClient::new(Some(dir.path().join("socket")));
        let id = client
            .checked(
                &[
                    "-f",
                    "/dev/null",
                    "new-session",
                    "-d",
                    "-s",
                    "capture-test",
                    "-x",
                    "80",
                    "-y",
                    "24",
                    "-P",
                    "-F",
                    "#{session_id}",
                    "printf 'WORKSPACE-CAPTURE-MARKER\\n\\\\001tmux-manager-section\\\\001\\nWORKSPACE-AFTER-MARKER\\n'; sleep 60",
                ])
            .await
            .unwrap();
        let session = SessionId::parse(id.trim()).unwrap();
        let workspace = Self {
            _dir: dir,
            client,
            session,
        };
        let dock = workspace
            .client
            .checked(&[
                "split-window",
                "-d",
                "-v",
                "-l",
                "4",
                "-t",
                workspace.session.as_str(),
                "-P",
                "-F",
                "#{pane_id}",
                "printf 'PROMPT-DOCK-EXCLUDED\\n'; sleep 60",
            ])
            .await
            .unwrap();
        workspace
            .client
            .checked(&[
                "set-option",
                "-p",
                "-t",
                dock.trim(),
                "@tmux_manager_dock",
                "fixture-owner",
            ])
            .await
            .unwrap();
        workspace
            .wait_for(workspace.session.as_str(), "WORKSPACE-CAPTURE-MARKER")
            .await;
        workspace
    }

    /// 等 target pane 印出 marker。
    async fn wait_for(&self, target: &str, marker: &str) {
        for _ in 0..100 {
            if self
                .client
                .checked(&["capture-pane", "-p", "-t", target])
                .await
                .unwrap()
                .contains(marker)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("{target} did not print {marker}");
    }

    /// 在 active pane 右側切出新 pane 並等它印出 marker。
    async fn split_right(&self, marker: &str) -> PaneId {
        let command = format!("printf '{marker}\\n'; sleep 60");
        let pane = self
            .client
            .checked(&[
                "split-window",
                "-d",
                "-h",
                "-t",
                self.session.as_str(),
                "-P",
                "-F",
                "#{pane_id}",
                &command,
            ])
            .await
            .unwrap();
        let pane = PaneId::parse(pane.trim()).unwrap();
        self.wait_for(pane.as_str(), marker).await;
        pane
    }

    async fn previews(&self, cache: &[PanePreview]) -> Vec<PanePreview> {
        self.client
            .snapshot(Some((&self.session, cache)))
            .await
            .unwrap()
            .preview
            .unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::process::Command::new("tmux")
            .arg("-S")
            .arg(self.client.socket.as_ref().unwrap())
            .arg("kill-server")
            .output();
    }
}

#[tokio::test]
#[ignore = "需要真實 tmux 與隔離 socket 權限"]
async fn real_tmux_batch_preview_and_capture_preserve_workspace_output() {
    let workspace = Workspace::new().await;
    let cache = workspace.previews(&[]).await;
    let snapshot = workspace
        .client
        .snapshot(Some((&workspace.session, &cache)))
        .await
        .unwrap();
    assert_eq!(snapshot.sessions.len(), 1);
    assert_eq!(texts(&snapshot.preview.unwrap()), [WORKSPACE_TEXT]);
    workspace.split_right("SECOND-WORKSPACE-MARKER").await;
    let panes = workspace
        .client
        .list_panes(&workspace.session)
        .await
        .unwrap();
    let ids: Vec<_> = panes.into_iter().map(|pane| pane.id).collect();
    assert_eq!(ids.len(), 2, "only the two work panes should be captured");
    assert_eq!(
        workspace.client.capture_many(&ids, 80).await.unwrap(),
        [WORKSPACE_TEXT, "SECOND-WORKSPACE-MARKER"]
    );
}

#[tokio::test]
#[ignore = "需要真實 tmux 與隔離 socket 權限"]
async fn real_tmux_snapshot_previews_every_pane_of_the_current_window() {
    let workspace = Workspace::new().await;
    workspace.split_right("RIGHT-PANE-MARKER").await;
    let first = workspace
        .client
        .snapshot(Some((&workspace.session, &[])))
        .await
        .unwrap();
    assert_eq!(first.sessions.len(), 1);
    let first = first.preview.unwrap();
    assert_eq!(first.len(), 2, "dock pane should be excluded: {first:?}");
    assert!(first[0].layout.left < first[1].layout.left);
    assert_eq!(first[0].layout.top, first[1].layout.top);
    // cache 是空的：所有 pane 都在同一次 snapshot 內補擷取。
    assert_eq!(texts(&first), [WORKSPACE_TEXT, "RIGHT-PANE-MARKER"]);
    let second = workspace.previews(&first).await;
    assert_eq!(texts(&second), [WORKSPACE_TEXT, "RIGHT-PANE-MARKER"]);
}

#[tokio::test]
#[ignore = "需要真實 tmux 與隔離 socket 權限"]
async fn real_tmux_snapshot_survives_a_closed_pane_in_the_cache() {
    let workspace = Workspace::new().await;
    let closed = workspace.split_right("CLOSED-PANE-MARKER").await;
    let mut cache = workspace.previews(&[]).await;
    workspace
        .client
        .checked(&["kill-pane", "-t", closed.as_str()])
        .await
        .unwrap();
    workspace.split_right("RIGHT-PANE-MARKER").await;
    // 已關閉的 pane 夾在中間：tmux 會在它的 capture 失敗後中止後續指令。
    cache.extend(workspace.previews(&[]).await.into_iter().skip(1));
    assert_eq!(cache.len(), 3);
    assert_eq!(cache[1].layout.id, closed);
    cache[2].text = "PREVIOUS-RIGHT-TEXT".into();
    let stale = workspace
        .client
        .snapshot(Some((&workspace.session, &cache)))
        .await
        .unwrap();
    assert_eq!(stale.sessions.len(), 1);
    let stale = stale.preview.unwrap();
    // 中止後沒擷取到的 pane 沿用上一輪文字。
    assert_eq!(texts(&stale), [WORKSPACE_TEXT, "PREVIOUS-RIGHT-TEXT"]);
    let fresh = workspace.previews(&stale).await;
    assert_eq!(texts(&fresh), [WORKSPACE_TEXT, "RIGHT-PANE-MARKER"]);
}

#[tokio::test]
#[ignore = "需要真實 tmux、隔離 socket 與本機 HTTP 權限"]
async fn real_tmux_ai_worker_sends_workspace_capture_and_returns_summary() {
    use std::io::{Read, Write};
    let workspace = Workspace::new().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "AI never sent a request"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("{error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        loop {
            let mut bytes = [0; 4096];
            let count = stream.read(&mut bytes).unwrap();
            assert!(count > 0, "incomplete HTTP request");
            request.extend_from_slice(&bytes[..count]);
            if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]);
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                if request.len() >= end + 4 + length {
                    let json: serde_json::Value =
                        serde_json::from_slice(&request[end + 4..end + 4 + length]).unwrap();
                    let content = json["messages"][0]["content"].as_str().unwrap();
                    assert!(content.contains("WORKSPACE-CAPTURE-MARKER"));
                    assert!(content.contains("\\001tmux-manager-section\\001"));
                    assert!(content.contains("WORKSPACE-AFTER-MARKER"));
                    assert!(!content.contains("PROMPT-DOCK-EXCLUDED"));
                    break;
                }
            }
        }
        let body =
            r#"{"content":[{"type":"text","text":"SUMMARY: 工作區擷取成功\nNAME: capture-ok"}]}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    let client = AiClient::new(Some("fixture-key".into()), "fixture-model", &endpoint).unwrap();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let sessions = workspace.client.list_sessions().await.unwrap();
    let mut service = AiService::new(client, workspace.client.clone(), sender);
    service.refresh(sessions, 7);
    let event = tokio::time::timeout(Duration::from_secs(8), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    let AppEvent::AiResult {
        session_id,
        generation,
        result,
    } = event
    else {
        panic!("unexpected event")
    };
    assert_eq!(session_id, workspace.session);
    assert_eq!(generation, 7);
    let summary = result.unwrap();
    assert_eq!(summary.summary, "工作區擷取成功");
    assert_eq!(summary.name, "capture-ok");
    server.join().unwrap();
}

/// tmux 依 client 的 LC_ALL/LC_CTYPE/LANG 判斷是否 UTF-8；非 UTF-8 時會把 tab 與非 ASCII 字元換成 `_`。
/// 測試不能安全地修改自身環境變數，因此移除 locale 後重新執行本測試，在子 process 內驗證。
#[tokio::test]
#[ignore = "需要真實 tmux 與隔離 socket 權限"]
async fn real_tmux_sessions_parse_without_utf8_locale() {
    const CHILD: &str = "TMUX_MANAGER_LOCALE_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "real_tmux_sessions_parse_without_utf8_locale",
                "--ignored",
                "--nocapture",
            ])
            .env_remove("LC_ALL")
            .env_remove("LC_CTYPE")
            .env_remove("LANG")
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success(), "非 UTF-8 locale 下解析 session 失敗");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("socket");
    let client = TmuxClient::new(Some(socket.clone()));
    client
        .checked(&["-f", "/dev/null", "new-session", "-d", "-s", "中文-locale"])
        .await
        .unwrap();
    let sessions = client.list_sessions().await;
    let _ = std::process::Command::new("tmux")
        .arg("-S")
        .arg(&socket)
        .arg("kill-server")
        .output();
    let names: Vec<_> = sessions.unwrap().into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["中文-locale"]);
}
