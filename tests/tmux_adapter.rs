use tmux_manager::tmux::{PaneId, PaneLayout, PanePreview, SessionId, TmuxClient};
mod support;
use std::sync::Arc;
use support::{preview::with_text, runner::FakeRunner};

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
    let pane = layout("%1", 0, 0, 80, 24, true);
    let output = format!(
        "$2\twork\t1\t5\n<BATCH_PRINTED_SECTION>\n{}<BATCH_PRINTED_SECTION>\nACTIVE OUTPUT\n",
        pane_line(&pane, false, "")
    );
    let client = TmuxClient::with_runner(None, Arc::new(FakeRunner::new(vec![(0, &output, "")])));
    let id = SessionId::parse("$2").unwrap();
    let snapshot = client
        .snapshot(Some((&id, &[blank(pane.clone())])))
        .await
        .unwrap();
    assert_eq!(snapshot.sessions.len(), 1);
    assert_eq!(
        snapshot.preview.unwrap(),
        [with_text(pane, "ACTIVE OUTPUT")]
    );
}
#[tokio::test]
async fn batch_preserves_old_marker_text_and_uses_a_fresh_marker_for_each_request() {
    let text = "before\n\\001tmux-manager-section\\001\nafter";
    let pane = layout("%1", 0, 0, 80, 24, true);
    let preview_output = format!(
        "$2\twork\t1\t5\n<BATCH_PRINTED_SECTION>\n{}<BATCH_PRINTED_SECTION>\n{text}\n",
        pane_line(&pane, false, "")
    );
    let capture_output =
        format!("{text}\n<BATCH_PRINTED_SECTION>\nother pane\n<BATCH_PRINTED_SECTION>\n");
    let runner = Arc::new(FakeRunner::new(vec![
        (0, &preview_output, ""),
        (0, &capture_output, ""),
    ]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let id = SessionId::parse("$2").unwrap();
    let preview = client
        .snapshot(Some((&id, &[blank(pane.clone())])))
        .await
        .unwrap()
        .preview
        .unwrap();
    assert_eq!(preview[0].text, text);
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
async fn snapshot_without_selection_is_a_plain_session_list() {
    let runner = Arc::new(FakeRunner::new(vec![(0, "$2\twork\t1\t5\n", "")]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let snapshot = client.snapshot(None).await.unwrap();
    assert_eq!((snapshot.sessions.len(), snapshot.preview), (1, None));
    assert_eq!(runner.calls.lock().unwrap()[0].args[0], "list-sessions");
    assert!(!runner.calls.lock().unwrap()[0].args.contains(&";".into()));
}
#[tokio::test]
async fn snapshot_keeps_sessions_without_preview_when_list_panes_fails() {
    let id = SessionId::parse("$2").unwrap();
    let cached = [blank(layout("%1", 0, 0, 80, 24, true))];
    // 實測 tmux：list-panes 失敗即中止後續指令，輸出停在第一個 sentinel。
    for cache in [&[][..], &cached[..]] {
        let out = format!("$2\twork\t1\t5\n{SENT}\n");
        let runner = Arc::new(FakeRunner::new(vec![(1, &out, "can't find session: $2")]));
        let snapshot = TmuxClient::with_runner(None, runner)
            .snapshot(Some((&id, cache)))
            .await
            .unwrap();
        assert_eq!((snapshot.sessions.len(), snapshot.preview), (1, None));
    }
}
#[tokio::test]
async fn snapshot_maps_missing_server_to_empty_and_other_errors_to_failure() {
    let id = SessionId::parse("$2").unwrap();
    let runner = Arc::new(FakeRunner::new(vec![(
        1,
        "",
        "no server running on /tmp/x",
    )]));
    let snapshot = TmuxClient::with_runner(None, runner)
        .snapshot(Some((&id, &[])))
        .await
        .unwrap();
    assert_eq!((snapshot.sessions.len(), snapshot.preview), (0, None));
    let runner = Arc::new(FakeRunner::new(vec![(1, "", "Permission denied")]));
    assert!(
        TmuxClient::with_runner(None, runner)
            .snapshot(Some((&id, &[])))
            .await
            .is_err()
    );
}
const PANE_FORMAT: &str = "#{pane_id}\t#{pane_index}\t#{pane_left}\t#{pane_top}\t#{pane_width}\t#{pane_height}\t#{pane_active}\t#{window_zoomed_flag}\t#{@tmux_manager_dock}\t#{pane_current_command}";
fn layout(id: &str, index: u32, left: u16, width: u16, height: u16, active: bool) -> PaneLayout {
    PaneLayout {
        id: PaneId::parse(id).unwrap(),
        index,
        left,
        top: 0,
        width,
        height,
        active,
        command: "zsh".into(),
    }
}
/// list-panes 的一行輸出；dock 為 `@tmux_manager_dock` 的值。
fn pane_line(pane: &PaneLayout, zoomed: bool, dock: &str) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{dock}\t{}\n",
        pane.id.as_str(),
        pane.index,
        pane.left,
        pane.top,
        pane.width,
        pane.height,
        u8::from(pane.active),
        u8::from(zoomed),
        pane.command
    )
}
fn blank(layout: PaneLayout) -> PanePreview {
    with_text(layout, "")
}
#[tokio::test]
async fn snapshot_lists_sessions_and_current_window_panes_in_one_invocation() {
    let left = layout("%1", 0, 0, 40, 24, true);
    let right = layout("%2", 1, 41, 39, 24, false);
    let out = format!(
        "$2\twork\t1\t5\n{SENT}\n{}{}",
        pane_line(&left, false, ""),
        pane_line(&right, false, "")
    );
    let runner = Arc::new(FakeRunner::new(vec![(0, &out, "")]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let id = SessionId::parse("$2").unwrap();
    let snapshot = client.snapshot(Some((&id, &[]))).await.unwrap();
    assert_eq!(snapshot.sessions.len(), 1);
    assert_eq!(snapshot.preview.unwrap(), [blank(left), blank(right)]);
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let tail = [
        ";",
        "display-message",
        "-p",
        &calls[0].args[6],
        ";",
        "list-panes",
        "-t",
        "$2:",
        "-F",
        PANE_FORMAT,
    ];
    assert_eq!(calls[0].args[3..], tail.map(String::from));
}
#[tokio::test]
async fn snapshot_captures_cached_panes_and_matches_text_by_pane_id() {
    let left = layout("%1", 0, 0, 40, 3, true);
    let new = layout("%3", 1, 41, 39, 12, false);
    let right = layout("%2", 2, 41, 39, 11, false);
    // cache 順序與本輪 list-panes 不同，且高度可能已變；擷取行數以 cache 為準。
    let cache = [
        blank(layout("%2", 1, 41, 39, 24, false)),
        blank(left.clone()),
    ];
    let lines = |prefix: &str| -> String { (1..=30).map(|i| format!("{prefix}{i}\n")).collect() };
    let out = format!(
        "$2\twork\t1\t5\n{SENT}\n{}{}{}{SENT}\n{}{SENT}\n{}",
        pane_line(&left, false, ""),
        pane_line(&new, false, ""),
        pane_line(&right, false, ""),
        lines("r"),
        lines("l"),
    );
    let runner = Arc::new(FakeRunner::new(vec![(0, &out, "")]));
    let client = TmuxClient::with_runner(None, runner.clone());
    let id = SessionId::parse("$2").unwrap();
    let preview = client
        .snapshot(Some((&id, &cache)))
        .await
        .unwrap()
        .preview
        .unwrap();
    let right_text: Vec<_> = (7..=30).map(|i| format!("r{i}")).collect();
    let expected = [
        with_text(left, "l28\nl29\nl30"),
        blank(new),
        with_text(right, &right_text.join("\n")),
    ];
    assert_eq!(preview, expected);
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let section = calls[0].args[6].clone();
    let captures: Vec<_> = [("%2", "-23"), ("%1", "-2")]
        .into_iter()
        .flat_map(|(pane, start)| {
            [";", "display-message", "-p", &section, ";"]
                .into_iter()
                .chain(["capture-pane", "-p", "-t", pane, "-S", start])
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(calls[0].args[13..], captures);
}
#[tokio::test]
async fn snapshot_keeps_panes_when_a_closed_cached_pane_aborts_the_captures() {
    let left = layout("%1", 0, 0, 40, 24, true);
    let right = layout("%2", 1, 41, 39, 24, false);
    let cache = [
        blank(left.clone()),
        blank(layout("%9", 1, 41, 39, 24, false)),
        blank(right.clone()),
    ];
    // 實測 tmux：capture-pane 失敗即中止後續指令，status 為 1，之前的輸出保留。
    let out = format!(
        "$2\twork\t1\t5\n{SENT}\n{}{}{SENT}\nLEFT\n{SENT}\n",
        pane_line(&left, false, ""),
        pane_line(&right, false, ""),
    );
    let runner = Arc::new(FakeRunner::new(vec![(1, &out, "can't find pane: %9")]));
    let id = SessionId::parse("$2").unwrap();
    let snapshot = TmuxClient::with_runner(None, runner)
        .snapshot(Some((&id, &cache)))
        .await
        .unwrap();
    assert_eq!(snapshot.sessions.len(), 1);
    let expected = [with_text(left, "LEFT"), blank(right)];
    assert_eq!(snapshot.preview.unwrap(), expected);
}
#[tokio::test]
async fn snapshot_keeps_cached_text_for_panes_whose_capture_did_not_run() {
    let left = layout("%1", 0, 0, 40, 24, true);
    let right = layout("%2", 1, 41, 39, 24, false);
    let cache = [
        with_text(left.clone(), "OLD LEFT"),
        with_text(layout("%9", 1, 41, 39, 24, false), "GONE"),
        with_text(right.clone(), "OLD RIGHT"),
    ];
    // 擷取到空白就是空白；%9 失敗後 %2 沒有擷取，沿用上一輪文字避免閃爍。
    let out = format!(
        "$2\twork\t1\t5\n{SENT}\n{}{}{SENT}\n\n{SENT}\n",
        pane_line(&left, false, ""),
        pane_line(&right, false, ""),
    );
    let runner = Arc::new(FakeRunner::new(vec![(1, &out, "can't find pane: %9")]));
    let id = SessionId::parse("$2").unwrap();
    let preview = TmuxClient::with_runner(None, runner)
        .snapshot(Some((&id, &cache)))
        .await
        .unwrap()
        .preview
        .unwrap();
    assert_eq!(preview, [blank(left), with_text(right, "OLD RIGHT")]);
}
#[tokio::test]
async fn snapshot_rejects_extra_sections_instead_of_misattributing_pane_output() {
    let left = layout("%1", 0, 0, 40, 24, true);
    let right = layout("%2", 1, 41, 39, 24, false);
    let out = format!(
        "$2\twork\t1\t5\n{SENT}\n{}{}{SENT}\nleft\n<BATCH_PRINTED_SECTION>\nstill left\n{SENT}\nright\n",
        pane_line(&left, false, ""),
        pane_line(&right, false, ""),
    );
    let runner = Arc::new(FakeRunner::new(vec![(0, &out, "")]));
    let id = SessionId::parse("$2").unwrap();
    assert!(
        TmuxClient::with_runner(None, runner)
            .snapshot(Some((&id, &[blank(left), blank(right)])))
            .await
            .is_err()
    );
}
async fn snapshot_preview(list_panes: &str) -> Vec<PanePreview> {
    let out = format!("$2\twork\t1\t5\n{SENT}\n{list_panes}");
    let runner = Arc::new(FakeRunner::new(vec![(0, &out, "")]));
    let id = SessionId::parse("$2").unwrap();
    TmuxClient::with_runner(None, runner)
        .snapshot(Some((&id, &[])))
        .await
        .unwrap()
        .preview
        .unwrap()
}
#[tokio::test]
async fn snapshot_excludes_prompt_dock_panes() {
    let work = layout("%1", 0, 0, 80, 19, true);
    let dock = layout("%2", 1, 0, 80, 4, false);
    let list = pane_line(&work, false, "") + &pane_line(&dock, false, "%1");
    assert_eq!(snapshot_preview(&list).await, [blank(work)]);
}
#[tokio::test]
async fn snapshot_keeps_only_the_active_pane_of_a_zoomed_window() {
    let left = layout("%1", 0, 0, 40, 24, false);
    let right = layout("%2", 1, 41, 39, 24, true);
    let list = pane_line(&left, true, "") + &pane_line(&right, true, "");
    assert_eq!(snapshot_preview(&list).await, [blank(right)]);
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
    let pane = layout("%1", 0, 0, 80, 40, true);
    let out = format!(
        "$2\twork\t1\t5\n{SENT}\n{}{SENT}\nfirst\nsecond\n{padding}",
        pane_line(&pane, false, "")
    );
    let runner = Arc::new(FakeRunner::new(vec![
        (0, &out, ""),
        (
            0,
            &format!("a1\n{padding}{SENT}\nb1\nb2\n{padding}{SENT}\n"),
            "",
        ),
    ]));
    let client = TmuxClient::with_runner(None, runner);
    let id = SessionId::parse("$2").unwrap();
    let preview = client
        .snapshot(Some((&id, &[blank(pane.clone())])))
        .await
        .unwrap()
        .preview
        .unwrap();
    assert_eq!(preview[0].text, "first\nsecond");
    let panes = [PaneId::parse("%1").unwrap(), PaneId::parse("%2").unwrap()];
    assert_eq!(
        client.capture_many(&panes, 5).await.unwrap(),
        ["a1", "b1\nb2"]
    );
}
#[tokio::test]
async fn unparsable_pane_lines_drop_only_the_preview() {
    let out = format!("$2\twork\t1\t5\n{SENT}\nnot a pane line\n");
    let runner = Arc::new(FakeRunner::new(vec![(0, &out, "")]));
    let id = SessionId::parse("$2").unwrap();
    let snapshot = TmuxClient::with_runner(None, runner)
        .snapshot(Some((&id, &[])))
        .await
        .unwrap();
    assert_eq!((snapshot.sessions.len(), snapshot.preview), (1, None));
}
