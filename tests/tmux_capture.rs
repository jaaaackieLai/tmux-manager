use std::time::Duration;
use tmux_manager::{
    ai::{AiClient, AiService},
    app::event::AppEvent,
    tmux::{SessionId, TmuxClient},
};

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
        for _ in 0..100 {
            if workspace
                .client
                .preview(&workspace.session)
                .await
                .unwrap()
                .contains("WORKSPACE-CAPTURE-MARKER")
            {
                return workspace;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("workspace did not print fixture output");
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
    let (sessions, preview) = workspace
        .client
        .snapshot(Some(&workspace.session))
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    let expected =
        "WORKSPACE-CAPTURE-MARKER\n\\001tmux-manager-section\\001\nWORKSPACE-AFTER-MARKER";
    assert_eq!(preview.as_deref(), Some(expected));
    let second = workspace
        .client
        .checked(&[
            "split-window",
            "-d",
            "-h",
            "-t",
            workspace.session.as_str(),
            "-P",
            "-F",
            "#{pane_id}",
            "printf 'SECOND-WORKSPACE-MARKER\\n'; sleep 60",
        ])
        .await
        .unwrap();
    let mut ready = false;
    for _ in 0..100 {
        if workspace
            .client
            .checked(&["capture-pane", "-p", "-t", second.trim()])
            .await
            .unwrap()
            .contains("SECOND-WORKSPACE-MARKER")
        {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(ready, "second pane did not print fixture output");
    let panes = workspace
        .client
        .list_panes(&workspace.session)
        .await
        .unwrap();
    let ids: Vec<_> = panes.into_iter().map(|pane| pane.id).collect();
    assert_eq!(ids.len(), 2, "only the two work panes should be captured");
    assert_eq!(
        workspace.client.capture_many(&ids, 80).await.unwrap(),
        [expected, "SECOND-WORKSPACE-MARKER"]
    );
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
