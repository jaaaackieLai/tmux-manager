use tmux_manager::tmux::{PaneId, SessionId, TmuxClient};
mod support;
use std::sync::Arc;
use support::runner::FakeRunner;

#[test]
fn pane_ids_cannot_be_names_or_command_text() {
    assert!(PaneId::parse("%12").is_ok());
    for value in ["", "%", "%1; kill-server", "session:0", "%-1", "%１２"] {
        assert!(PaneId::parse(value).is_err(), "accepted {value}");
    }
}
#[tokio::test]
async fn absent_server_is_an_empty_session_list() {
    let runner = Arc::new(FakeRunner::new(vec![(
        1,
        "",
        "no server running on /tmp/test",
    )]));
    assert!(
        TmuxClient::with_runner(None, runner)
            .list_sessions()
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn session_names_are_data_and_targets_are_ids() {
    let runner = Arc::new(FakeRunner::new(vec![
        (0, "$2\t中文 $(touch /tmp/not-created)\t2\t123\n", ""),
        (0, "", ""),
    ]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let sessions = client.list_sessions().await.unwrap();
    assert_eq!(sessions[0].name, "中文 $(touch /tmp/not-created)");
    client
        .rename(&SessionId::parse("$2").unwrap(), "has ' quotes; $(noop)")
        .await
        .unwrap();
    let calls = runner.calls.lock().unwrap();
    assert_eq!(
        calls[1].args,
        ["rename-session", "-t", "$2", "has ' quotes; $(noop)"]
    );
}
#[tokio::test]
async fn permission_errors_are_not_hidden_as_empty_lists() {
    let runner = Arc::new(FakeRunner::new(vec![(1, "", "Permission denied")]));
    assert!(
        TmuxClient::with_runner(None, runner)
            .list_sessions()
            .await
            .is_err()
    );
}
#[test]
fn help_and_version_do_not_need_tmux() {
    for flag in ["--help", "--version"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_tmux-manager"))
            .arg(flag)
            .env("PATH", "/nonexistent")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("tmux-manager"));
    }
}
#[tokio::test]
async fn minimum_version_checks_and_popup_capability_are_explicit() {
    for (version, supported) in [
        ("tmux 3.4\n", true),
        ("tmux 3.7c\n", true),
        ("tmux 3.2a\n", false),
        ("unknown", false),
    ] {
        let runner = Arc::new(FakeRunner::new(vec![(0, version, "")]));
        assert_eq!(
            TmuxClient::with_runner(None, runner)
                .check_version()
                .await
                .is_ok(),
            supported
        );
    }
    let runner = Arc::new(FakeRunner::new(vec![(
        0,
        "list-sessions [-F format]\ndisplay-popup [-E] [shell-command]\n",
        "",
    )]));
    assert!(
        TmuxClient::with_runner(None, runner)
            .supports_popup()
            .await
            .unwrap()
    );
}
const SENT: &str = "<BATCH_SECTION>";
#[tokio::test]
async fn capture_many_rejects_extra_sections_instead_of_misattributing_pane_output() {
    let output = format!("before\n<BATCH_PRINTED_SECTION>\nafter\n{SENT}\nother pane\n{SENT}\n");
    let client = TmuxClient::with_runner(None, Arc::new(FakeRunner::new(vec![(0, &output, "")])));
    let panes = [PaneId::parse("%1").unwrap(), PaneId::parse("%2").unwrap()];
    assert!(client.capture_many(&panes, 80).await.is_err());
}
#[tokio::test]
async fn capture_many_parses_tmux_printable_control_character_separators() {
    let output = "pane one\n<BATCH_PRINTED_SECTION>\npane two\n<BATCH_PRINTED_SECTION>\n";
    let client = TmuxClient::with_runner(None, Arc::new(FakeRunner::new(vec![(0, output, "")])));
    let panes = [PaneId::parse("%1").unwrap(), PaneId::parse("%2").unwrap()];
    assert_eq!(
        client.capture_many(&panes, 80).await.unwrap(),
        ["pane one", "pane two"]
    );
}
#[tokio::test]
async fn snapshot_parses_tmux_printable_control_character_separator() {
    let output = "$2\twork\t1\t5\n<BATCH_PRINTED_SECTION>\nACTIVE OUTPUT\n";
    let client = TmuxClient::with_runner(None, Arc::new(FakeRunner::new(vec![(0, output, "")])));
    let (sessions, preview) = client
        .snapshot(Some(&SessionId::parse("$2").unwrap()))
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(preview.as_deref(), Some("ACTIVE OUTPUT"));
}
#[tokio::test]
async fn batch_preserves_old_marker_text_and_uses_a_fresh_marker_for_each_request() {
    let text = "before\n\\001tmux-manager-section\\001\nafter";
    let preview_output = format!("$2\twork\t1\t5\n<BATCH_PRINTED_SECTION>\n{text}\n");
    let capture_output =
        format!("{text}\n<BATCH_PRINTED_SECTION>\nother pane\n<BATCH_PRINTED_SECTION>\n");
    let runner = Arc::new(FakeRunner::new(vec![
        (0, &preview_output, ""),
        (0, &capture_output, ""),
    ]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let (_, preview) = client
        .snapshot(Some(&SessionId::parse("$2").unwrap()))
        .await
        .unwrap();
    assert_eq!(preview.as_deref(), Some(text));
    let panes = [PaneId::parse("%1").unwrap(), PaneId::parse("%2").unwrap()];
    assert_eq!(
        client.capture_many(&panes, 80).await.unwrap(),
        [text, "other pane"]
    );
    let calls = runner.calls.lock().unwrap();
    let markers: Vec<_> = calls
        .iter()
        .map(|call| {
            &call
                .args
                .windows(3)
                .find(|args| args[0] == "display-message" && args[1] == "-p")
                .unwrap()[2]
        })
        .collect();
    assert_ne!(markers[0], markers[1]);
}
#[tokio::test]
async fn preview_captures_the_session_target_which_is_its_active_pane() {
    let runner = Arc::new(FakeRunner::new(vec![(0, "ACTIVE OUTPUT\n", "")]));
    let client = TmuxClient::with_runner(None, runner.clone());
    assert_eq!(
        client
            .preview(&SessionId::parse("$2").unwrap())
            .await
            .unwrap(),
        "ACTIVE OUTPUT"
    );
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].args,
        ["capture-pane", "-p", "-t", "$2", "-S", "-14"]
    );
}
#[tokio::test]
async fn snapshot_lists_sessions_and_previews_active_pane_in_one_invocation() {
    let capture: String = (1..=20).map(|i| format!("line {i}\n")).collect();
    let out = format!("$2\twork\t1\t5\n$3\tother\t2\t6\n{SENT}\n{capture}");
    let runner = Arc::new(FakeRunner::new(vec![(0, &out, "")]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let id = SessionId::parse("$2").unwrap();
    let (sessions, preview) = client.snapshot(Some(&id)).await.unwrap();
    assert_eq!(sessions.len(), 2);
    let preview = preview.unwrap();
    assert_eq!(preview.lines().count(), 15);
    assert!(preview.starts_with("line 6") && preview.ends_with("line 20"));
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let tail = [
        ";",
        "display-message",
        "-p",
        &calls[0].args[6],
        ";",
        "capture-pane",
        "-p",
        "-t",
        "$2",
        "-S",
        "-14",
    ];
    assert_eq!(calls[0].args[3..], tail.map(String::from));
}
#[tokio::test]
async fn snapshot_without_selection_is_a_plain_session_list() {
    let runner = Arc::new(FakeRunner::new(vec![(0, "$2\twork\t1\t5\n", "")]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let (sessions, preview) = client.snapshot(None).await.unwrap();
    assert_eq!((sessions.len(), preview), (1, None));
    assert_eq!(runner.calls.lock().unwrap()[0].args[0], "list-sessions");
    assert!(!runner.calls.lock().unwrap()[0].args.contains(&";".into()));
}
#[tokio::test]
async fn snapshot_keeps_sessions_when_only_the_capture_fails() {
    let out = format!("$2\twork\t1\t5\n{SENT}\n");
    let runner = Arc::new(FakeRunner::new(vec![(1, &out, "can't find pane: $2")]));
    let client = TmuxClient::with_runner(None, runner);
    let (sessions, preview) = client
        .snapshot(Some(&SessionId::parse("$2").unwrap()))
        .await
        .unwrap();
    assert_eq!((sessions.len(), preview), (1, None));
}
#[tokio::test]
async fn snapshot_maps_missing_server_to_empty_and_other_errors_to_failure() {
    let id = SessionId::parse("$2").unwrap();
    let runner = Arc::new(FakeRunner::new(vec![(
        1,
        "",
        "no server running on /tmp/x",
    )]));
    let (sessions, preview) = TmuxClient::with_runner(None, runner)
        .snapshot(Some(&id))
        .await
        .unwrap();
    assert_eq!((sessions.len(), preview), (0, None));
    let runner = Arc::new(FakeRunner::new(vec![(1, "", "Permission denied")]));
    assert!(
        TmuxClient::with_runner(None, runner)
            .snapshot(Some(&id))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn capture_many_splits_one_invocation_into_per_pane_sections() {
    let out = format!("a1\na2\na3\n{SENT}\nb1\n{SENT}\n");
    let runner = Arc::new(FakeRunner::new(vec![(0, &out, "")]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let panes = [PaneId::parse("%1").unwrap(), PaneId::parse("%2").unwrap()];
    assert_eq!(
        client.capture_many(&panes, 2).await.unwrap(),
        ["a2\na3", "b1"]
    );
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0]
            .args
            .iter()
            .filter(|a| *a == "capture-pane")
            .count(),
        2
    );
    assert!(calls[0].args.windows(2).any(|w| w == ["-S", "-1"]));
}
#[tokio::test]
async fn capture_many_rejects_truncated_output() {
    let runner = Arc::new(FakeRunner::new(vec![(0, &format!("a1\n{SENT}\n"), "")]));
    let panes = [PaneId::parse("%1").unwrap(), PaneId::parse("%2").unwrap()];
    assert!(
        TmuxClient::with_runner(None, runner)
            .capture_many(&panes, 2)
            .await
            .is_err()
    );
    assert!(
        TmuxClient::with_runner(None, Arc::new(FakeRunner::new(vec![])))
            .capture_many(&[], 2)
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn captures_skip_blank_rows_that_pad_the_pane_height() {
    let padding = "\n".repeat(38);
    let out = format!("$2\twork\t1\t5\n{SENT}\nfirst\nsecond\n{padding}");
    let runner = Arc::new(FakeRunner::new(vec![
        (0, &out, ""),
        (
            0,
            &format!("a1\n{padding}{SENT}\nb1\nb2\n{padding}{SENT}\n"),
            "",
        ),
    ]));
    let client = TmuxClient::with_runner(None, runner);
    let (_, preview) = client
        .snapshot(Some(&SessionId::parse("$2").unwrap()))
        .await
        .unwrap();
    assert_eq!(preview.unwrap(), "first\nsecond");
    let panes = [PaneId::parse("%1").unwrap(), PaneId::parse("%2").unwrap()];
    assert_eq!(
        client.capture_many(&panes, 5).await.unwrap(),
        ["a1", "b1\nb2"]
    );
}
