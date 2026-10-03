mod support;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend};
use std::sync::Arc;
use support::runner::FakeRunner;
use tmux_manager::{
    prompts::{dock_ui::DockView, model::PromptSlot},
    tmux::{PaneId, TmuxClient},
};

/// 底部列第二列（prompt 按鈕列）上的滑鼠事件。
fn event(kind: MouseEventKind) -> MouseEvent {
    MouseEvent {
        kind,
        column: 3,
        row: 1,
        modifiers: KeyModifiers::NONE,
    }
}

fn dock_terminal() -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(80, 4)).unwrap()
}

#[tokio::test]
async fn clicking_the_dock_targets_last_work_pane_and_never_the_dock_itself() {
    let runner = Arc::new(FakeRunner::new(vec![(
        0,
        "%1\t0\t0\t\n%2\t0\t1\t\n%3\t1\t0\towned\n",
        "",
    )]));
    let tmux = TmuxClient::with_runner(Some("/tmp/fixture.socket".into()), runner.clone());
    let target = tmux
        .dock_target(&PaneId::parse("%3").unwrap())
        .await
        .unwrap();
    assert_eq!(target.as_str(), "%2");
    let calls = runner.calls.lock().unwrap();
    assert_eq!(&calls[0].args[..4], &["list-panes", "-t", "%3", "-F"]);
}

#[tokio::test]
async fn dock_target_uses_current_active_work_pane_and_refuses_arbitrary_fallback() {
    for (text, expected) in [
        ("%1\t1\t0\t\n%2\t0\t1\t\n%3\t0\t0\towned\n", Some("%1")),
        ("%1\t0\t0\t\n%3\t1\t1\towned\n", None),
        ("%2\t0\t1\tother-dock\n%3\t1\t0\towned\n", None),
    ] {
        let tmux = TmuxClient::with_runner(None, Arc::new(FakeRunner::new(vec![(0, text, "")])));
        let result = tmux.dock_target(&PaneId::parse("%3").unwrap()).await;
        assert_eq!(result.ok().as_ref().map(PaneId::as_str), expected);
    }
}

#[test]
fn mouse_buttons_use_rendered_prompt_ids_and_release_drag_right_click_never_paste() {
    let slots = vec![
        PromptSlot::new("中文🙂", "first", vec![], 0).unwrap(),
        PromptSlot::new("second", "next", vec![], 1).unwrap(),
    ];
    let mut view = DockView::default();
    let mut terminal = dock_terminal();
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    assert_eq!(
        view.handle_mouse(event(MouseEventKind::Down(MouseButton::Left))),
        Some(slots[0].id)
    );
    for kind in [
        MouseEventKind::Up(MouseButton::Left),
        MouseEventKind::Drag(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
    ] {
        assert_eq!(view.handle_mouse(event(kind)), None);
    }
}

#[test]
fn mouse_paging_tracks_uuid_and_handles_tiny_unicode_terminals() {
    let slots: Vec<_> = (0..8)
        .map(|i| PromptSlot::new(&format!("中文🙂-{i}"), "body", vec![], i).unwrap())
        .collect();
    let mut view = DockView::default();
    let mut terminal = Terminal::new(TestBackend::new(24, 4)).unwrap();
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    let first = view
        .handle_mouse(event(MouseEventKind::Down(MouseButton::Left)))
        .unwrap();
    view.handle_mouse(event(MouseEventKind::ScrollDown));
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    assert_ne!(
        view.handle_mouse(event(MouseEventKind::Down(MouseButton::Left)))
            .unwrap(),
        first
    );
    view.handle_mouse(event(MouseEventKind::ScrollUp));
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    assert_eq!(
        view.handle_mouse(event(MouseEventKind::Down(MouseButton::Left))),
        Some(first)
    );
    for (w, h) in [(1, 1), (1, 4), (8, 4), (40, 4)] {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    }
}

#[test]
fn narrow_dock_keeps_the_buffer_name_visible_when_multiline_is_not_pasted() {
    let mut terminal = Terminal::new(TestBackend::new(40, 4)).unwrap();
    let mut view = DockView::default();
    terminal
        .draw(|f| {
            view.render(
                f,
                &[],
                "多行尚未貼上 · tmux-manager-12345678-1234-1234-1234-123456789abc",
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let last: String = (0..40).map(|x| buffer[(x, 3)].symbol()).collect();
    assert!(
        last.contains("789abc"),
        "buffer name was clipped from the narrow dock: {last}"
    );
}

#[test]
fn wheel_scroll_down_keeps_every_prompt_when_all_fit() {
    let slots: Vec<_> = ["a", "b", "c"]
        .iter()
        .enumerate()
        .map(|(i, t)| PromptSlot::new(t, "body", vec![], i as u32).unwrap())
        .collect();
    let mut view = DockView::default();
    let mut terminal = dock_terminal();
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    view.handle_mouse(event(MouseEventKind::ScrollDown));
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    let row = support::render::row_text(&terminal, 1);
    for title in ["[ a ]", "[ b ]", "[ c ]"] {
        assert!(row.contains(title), "{title} missing from {row:?}");
    }
}

#[test]
fn paging_back_returns_to_the_previous_page_even_when_widths_differ() {
    // 第 1 頁放得下 4 個中等標題，第 2 頁只放得下 2 個長標題。
    let titles = [
        "medium-0000",
        "medium-1111",
        "medium-2222",
        "medium-3333",
        "a-very-long-title-44",
        "a-very-long-title-55",
        "a-very-long-title-66",
    ];
    let slots: Vec<_> = titles
        .iter()
        .enumerate()
        .map(|(i, t)| PromptSlot::new(t, "body", vec![], i as u32).unwrap())
        .collect();
    let mut view = DockView::default();
    let mut terminal = dock_terminal();
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    view.handle_mouse(event(MouseEventKind::ScrollDown));
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    view.handle_mouse(event(MouseEventKind::ScrollUp));
    terminal.draw(|f| view.render(f, &slots, "ready")).unwrap();
    assert_eq!(
        view.handle_mouse(event(MouseEventKind::Down(MouseButton::Left))),
        Some(slots[0].id)
    );
}

#[test]
#[cfg(unix)]
fn one_server_reached_through_a_symlinked_socket_path_shares_one_dock_lock() {
    use tmux_manager::{prompts::dock_session::lock_path, tmux::SessionId};
    // 同一個 session 只能有一個 supervisor；路徑寫法不同時兩邊都會把對方的底部列當殘留刪除。
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    std::fs::create_dir(&real).unwrap();
    std::fs::write(real.join("socket"), "").unwrap();
    std::os::unix::fs::symlink(&real, dir.path().join("link")).unwrap();
    let session = SessionId::parse("$1").unwrap();
    assert_eq!(
        lock_path(&real.join("socket"), &session),
        lock_path(&dir.path().join("link/socket"), &session)
    );
}

#[test]
fn the_dock_pane_command_uses_absolute_paths() {
    use std::path::Path;
    use tmux_manager::prompts::dock_session::dock_command;
    // tmux 在工作 pane 的 cwd 啟動底部列；相對路徑會指到別的設定檔或 socket。
    let command = dock_command(
        Path::new("/opt/tmux-manager"),
        Path::new("run/sock"),
        Path::new("conf/config.toml"),
    )
    .unwrap();
    let cwd = std::env::current_dir().unwrap();
    assert!(
        command.contains(&cwd.join("run/sock").to_string_lossy().into_owned()),
        "{command:?}"
    );
    assert!(
        command.contains(&cwd.join("conf/config.toml").to_string_lossy().into_owned()),
        "{command:?}"
    );
}
