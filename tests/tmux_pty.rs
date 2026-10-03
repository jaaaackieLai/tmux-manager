use std::{
    fs,
    time::{Duration, Instant},
};
use tmux_manager::{
    prompts::paste::{PasteOutcome, PasteService, PasteTarget},
    tmux::{PaneId, TmuxClient},
};
struct Fixture {
    dir: tempfile::TempDir,
    client: TmuxClient,
    first: PaneId,
    second: PaneId,
}
impl Fixture {
    async fn new(bracket: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("socket");
        let client = TmuxClient::new(Some(socket));
        let helper = dir.path().join("reader.py");
        fs::write(
            &helper,
            r#"import os, sys, tty, time
fd = sys.stdin.fileno()
tty.setraw(fd)
os.write(1, b'\x1b[?2004h' if sys.argv[2]=='on' else b'\x1b[?2004l')
open(sys.argv[1]+'.ready','w').close()
with open(sys.argv[1],'wb',buffering=0) as output:
 while True:
  data = os.read(fd,65536)
  if not data: break
  output.write(data)
"#,
        )
        .unwrap();
        let mode = if bracket { "on" } else { "off" };
        let command = format!(
            "python3 '{}' '{}' {mode}",
            helper.display(),
            dir.path().join("first").display()
        );
        let first = client
            .checked(&[
                "-f",
                "/dev/null",
                "new-session",
                "-d",
                "-s",
                "fixture",
                "-x",
                "80",
                "-y",
                "24",
                "-P",
                "-F",
                "#{pane_id}",
                &command,
            ])
            .await
            .unwrap();
        let first = PaneId::parse(first.trim()).unwrap();
        let command = format!(
            "python3 '{}' '{}' {mode}",
            helper.display(),
            dir.path().join("second").display()
        );
        let second = client
            .checked(&[
                "split-window",
                "-h",
                "-t",
                first.as_str(),
                "-P",
                "-F",
                "#{pane_id}",
                &command,
            ])
            .await
            .unwrap();
        let second = PaneId::parse(second.trim()).unwrap();
        for file in ["first.ready", "second.ready"] {
            wait_for(|| dir.path().join(file).exists());
        }
        Self {
            dir,
            client,
            first,
            second,
        }
    }
    fn bytes(&self, name: &str) -> Vec<u8> {
        fs::read(self.dir.path().join(name)).unwrap_or_default()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::process::Command::new("tmux")
            .arg("-S")
            .arg(self.client.socket.as_ref().unwrap())
            .arg("kill-server")
            .output();
    }
}
fn wait_for(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(Instant::now() < deadline, "fixture timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[tokio::test]
#[ignore = "需要 tmux、python3 與建立 PTY/socket 權限；CI 必須另行執行 --ignored"]
async fn real_tmux_paste_is_byte_exact_and_never_broadcasts_to_synchronized_panes() {
    let fixture = Fixture::new(true).await;
    fixture
        .client
        .checked(&[
            "set-window-option",
            "-t",
            fixture.first.as_str(),
            "synchronize-panes",
            "on",
        ])
        .await
        .unwrap();
    let body = "中文🙂\n'quotes'\n尾端\n";
    // 驗證 tmux 3.3+ paste-buffer 的底層契約；產品服務仍須確認能力才直貼多行。
    fixture
        .client
        .checked_with_stdin(
            &["load-buffer", "-b", "fixture-buffer", "-"],
            body.as_bytes(),
        )
        .await
        .unwrap();
    fixture
        .client
        .checked(&[
            "paste-buffer",
            "-p",
            "-r",
            "-b",
            "fixture-buffer",
            "-t",
            fixture.first.as_str(),
        ])
        .await
        .unwrap();
    let expected = format!("\x1b[200~{body}\x1b[201~").into_bytes();
    wait_for(|| fixture.bytes("first").len() >= expected.len());
    assert_eq!(fixture.bytes("first"), expected);
    assert!(fixture.bytes("second").is_empty());
    let caps = fixture.client.capabilities(&fixture.first).await.unwrap();
    let outcome = PasteService::new(fixture.client.clone())
        .paste(
            &PasteTarget {
                socket: fixture.client.socket.clone(),
                pane_id: fixture.first.clone(),
            },
            body,
        )
        .await
        .unwrap();
    if caps.bracketed_paste == Some(true) {
        assert!(matches!(outcome, PasteOutcome::Delivered { .. }));
        wait_for(|| fixture.bytes("first").len() >= expected.len() * 2);
        assert_eq!(
            fixture.bytes("first"),
            [expected.clone(), expected].concat()
        );
    } else {
        let PasteOutcome::BufferedOnly { buffer, .. } = outcome else {
            panic!("must buffer when capability unknown")
        };
        assert_eq!(
            fixture
                .client
                .run(&["save-buffer", "-b", &buffer, "-"], &[], false)
                .await
                .unwrap()
                .stdout,
            body.as_bytes()
        );
    }
    assert!(fixture.bytes("second").is_empty());
    assert_ne!(fixture.first, fixture.second);
}
#[tokio::test]
#[ignore = "需要 tmux、python3 與建立 PTY/socket 權限；CI 必須另行執行 --ignored"]
async fn real_tmux_unknown_or_disabled_paste_buffers_multiline_but_delivers_singleline() {
    let fixture = Fixture::new(false).await;
    let service = PasteService::new(fixture.client.clone());
    let target = PasteTarget {
        socket: fixture.client.socket.clone(),
        pane_id: fixture.first.clone(),
    };
    assert!(matches!(
        service.paste(&target, "多行\n尾端\n").await.unwrap(),
        PasteOutcome::BufferedOnly { .. }
    ));
    assert!(fixture.bytes("first").is_empty());
    assert!(matches!(
        service.paste(&target, "單行🙂").await.unwrap(),
        PasteOutcome::Delivered { .. }
    ));
    wait_for(|| !fixture.bytes("first").is_empty());
    assert_eq!(fixture.bytes("first"), "單行🙂".as_bytes());
    assert!(fixture.bytes("second").is_empty());
}
#[tokio::test]
#[ignore = "需要 tmux、python3 與建立 PTY/socket 權限；CI 必須另行執行 --ignored"]
async fn real_tmux_rejects_closed_pane_instead_of_using_active_pane() {
    let fixture = Fixture::new(true).await;
    fixture
        .client
        .checked(&["kill-pane", "-t", fixture.first.as_str()])
        .await
        .unwrap();
    assert!(
        PasteService::new(fixture.client.clone())
            .paste(
                &PasteTarget {
                    socket: fixture.client.socket.clone(),
                    pane_id: fixture.first.clone()
                },
                "must not arrive"
            )
            .await
            .is_err()
    );
    assert!(fixture.bytes("second").is_empty());
}
#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_pty_manager_editor_restart_resize_ctrl_c_and_terminal_restore() {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/terminal_smoke.py"
        ))
        .arg(env!("CARGO_BIN_EXE_tmux-manager"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_popup_binding_mouse_click_and_original_target_with_spaced_binary_path() {
    let output = std::process::Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/popup_smoke.py"))
        .arg(env!("CARGO_BIN_EXE_tmux-manager"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_prompt_dock_mouse_only_active_target_and_detach_cleanup() {
    let output = std::process::Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/dock_smoke.py"))
        .arg(env!("CARGO_BIN_EXE_tmux-manager"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_dock_pty(script: &str, args: &[&str]) {
    let output = std::process::Command::new("python3")
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join(script),
        )
        .arg(env!("CARGO_BIN_EXE_tmux-manager"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_prompt_dock_slow_start_and_startup_termination_clean_owned_resources() {
    run_dock_pty("dock_start_smoke.py", &[]);
    run_dock_pty("dock_start_smoke.py", &["--cancel"]);
}
#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_prompt_dock_survives_inside_manager_exit_and_spaced_binary_path() {
    run_dock_pty("dock_inside_smoke.py", &[]);
}
#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_prompt_dock_survives_hangup_and_replaces_a_dead_owners_bars() {
    run_dock_pty("dock_orphan_smoke.py", &[]);
}
#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_prompt_dock_keeps_bars_through_transient_tmux_failures() {
    run_dock_pty("dock_transient_smoke.py", &[]);
}
#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_prompt_dock_reattach_during_old_cleanup_still_gets_bars() {
    run_dock_pty("dock_handoff_smoke.py", &[]);
}
#[test]
#[ignore = "需要 tmux、python3 與 PTY/socket；CI 必須另行執行 --ignored"]
fn real_prompt_dock_closes_a_window_whose_last_work_pane_exited() {
    run_dock_pty("dock_last_pane_smoke.py", &[]);
}
