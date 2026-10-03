use tmux_manager::ai::{AiClient, parse_response};
#[test]
fn multiple_content_blocks_are_parsed_and_missing_fields_rejected() {
    let result = parse_response(&serde_json::json!({"content": [{"type":"text","text":"SUMMARY: 正在修測試"},{"type":"text","text":"NAME: fix-tests"}]})).unwrap();
    assert_eq!(result.summary, "正在修測試");
    assert_eq!(result.name, "fix-tests");
    assert!(parse_response(&serde_json::json!({"content":[]})).is_err());
    assert!(
        parse_response(&serde_json::json!({"content":[{"type":"text","text":"SUMMARY: only"}]}))
            .is_err()
    );
}
#[tokio::test]
async fn no_key_makes_no_http_request() {
    let client = AiClient::new(None, "model", "http://127.0.0.1:1").unwrap();
    assert!(client.summarize("text").await.unwrap().is_none());
}
fn server(status: &str, body: &str) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{body}",
        body.len()
    );
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0; 8192];
        let _ = stream.read(&mut buf);
        stream.write_all(response.as_bytes()).unwrap();
    });
    (url, thread)
}
#[tokio::test]
async fn http_success_and_errors_are_visible_without_retries() {
    for (status, body, success) in [
        (
            "200 OK",
            r#"{"content":[{"type":"text","text":"SUMMARY: 開發中\nNAME: dev"}]}"#,
            true,
        ),
        ("401 Unauthorized", "{}", false),
        ("429 Too Many Requests", "{}", false),
        ("200 OK", "broken", false),
    ] {
        let (url, thread) = server(status, body);
        let client = AiClient::new(Some("test-key".into()), "model", &url).unwrap();
        assert_eq!(client.summarize("中文 output").await.is_ok(), success);
        thread.join().unwrap();
    }
}
#[tokio::test]
async fn ai_workers_never_exceed_four_simultaneous_requests() {
    use std::{
        io::{Read, Write},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tmux_manager::{
        ai::AiService,
        app::event::AppEvent,
        tmux::{
            Session, SessionId, TmuxClient,
            command::{BoxFuture, CommandOutput, CommandRequest, Runner},
        },
    };
    struct Capture;
    impl Runner for Capture {
        fn run(
            &self,
            request: CommandRequest,
        ) -> BoxFuture<'_, tmux_manager::Result<CommandOutput>> {
            let stdout = if request.args[0] == "list-panes" {
                "%1\twork\n".as_bytes().to_vec()
            } else {
                let section = request
                    .args
                    .windows(3)
                    .find(|args| args[0] == "display-message" && args[1] == "-p")
                    .unwrap();
                format!(
                    "terminal output\n{}\n",
                    section[2].replace('\u{1}', "\\001")
                )
                .into_bytes()
            };
            Box::pin(std::future::ready(Ok(CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            })))
        }
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let maximum = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let max = maximum.clone();
    let server = std::thread::spawn(move || {
        let mut handlers = Vec::new();
        for _ in 0..8 {
            let (mut stream, _) = listener.accept().unwrap();
            let active = active.clone();
            let maximum = max.clone();
            handlers.push(std::thread::spawn(move || {
                let mut buf=[0;8192];let _=stream.read(&mut buf);
                let count=active.fetch_add(1,Ordering::SeqCst)+1;maximum.fetch_max(count,Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(80));
                let body=r#"{"content":[{"type":"text","text":"SUMMARY: 測試\nNAME: test"}]}"#;
                let response=format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{body}",body.len());
                active.fetch_sub(1,Ordering::SeqCst);stream.write_all(response.as_bytes()).unwrap();
            }));
        }
        for handler in handlers {
            handler.join().unwrap();
        }
    });
    let client = AiClient::new(Some("test".into()), "model", &endpoint).unwrap();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut service = AiService::new(
        client,
        TmuxClient::with_runner(None, Arc::new(Capture)),
        sender,
    );
    service.refresh(
        (0..8)
            .map(|i| Session {
                id: SessionId::parse(&format!("${i}")).unwrap(),
                name: format!("session-{i}"),
                windows: 1,
                created: 0,
            })
            .collect(),
        1,
    );
    for _ in 0..8 {
        let message = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        let AppEvent::AiResult {
            generation, result, ..
        } = message
        else {
            panic!("unexpected event")
        };
        assert_eq!(generation, 1);
        assert!(result.is_ok());
    }
    server.join().unwrap();
    assert!(maximum.load(Ordering::SeqCst) <= 4);
    assert!(maximum.load(Ordering::SeqCst) >= 2);
}
#[tokio::test]
async fn http_request_times_out_at_fifteen_seconds_without_retry() {
    use std::{
        io::Read,
        time::{Duration, Instant},
    };
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0; 8192];
        let _ = stream.read(&mut buf);
        std::thread::sleep(Duration::from_secs(16));
    });
    let client = AiClient::new(Some("test".into()), "model", &endpoint).unwrap();
    let start = Instant::now();
    let error = client.summarize("capture").await.unwrap_err();
    assert!(matches!(error,tmux_manager::Error::Http(ref e) if e.is_timeout()));
    assert!(start.elapsed() >= Duration::from_secs(14));
    assert!(start.elapsed() < Duration::from_secs(18));
    server.join().unwrap();
}
