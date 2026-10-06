use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{Terminal, backend::TestBackend};
use tmux_manager::{
    app::{AppState, ManagerAction, Screen},
    tmux::{Session, SessionId},
    ui::manager_mouse::ManagerHitMap,
};

mod support;

fn preview_marker() -> tmux_manager::tmux::PanePreview {
    support::preview::pane(0, 0, 80, true, "zsh", "PREVIEW-MARKER")
}

fn app() -> AppState {
    let mut app = AppState::default();
    app.replace_sessions(
        (1..=2)
            .map(|id| Session {
                id: SessionId::parse(&format!("${id}")).unwrap(),
                name: format!("session-{id}"),
                windows: 1,
                created: 123,
            })
            .collect(),
    );
    app.preview = vec![preview_marker()];
    app
}

fn render(app: &AppState, width: u16, height: u16) -> (Terminal<TestBackend>, ManagerHitMap) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut hits = ManagerHitMap::default();
    terminal
        .draw(|frame| hits = tmux_manager::ui::manager::render(frame, app))
        .unwrap();
    (terminal, hits)
}

fn text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        let mut x = 0;
        while x < buffer.area.width {
            let symbol = buffer[(x, y)].symbol();
            text.push_str(symbol);
            x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
        }
        text.push('\n');
    }
    text
}

#[test]
fn action_menu_keeps_list_and_preview_visible() {
    let mut app = app();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let (terminal, _) = render(&app, 80, 24);
    let screen = text(&terminal);
    assert!(
        screen.contains("Sessions"),
        "action menu replaced the session list"
    );
    assert!(screen.contains("Preview"), "action menu replaced Preview");
    assert!(
        screen.contains("session-2"),
        "background session disappeared"
    );
    assert!(screen.contains("attach"));
    let (_, hits) = render(&app, 80, 24);
    let menu = hits.action_menu.unwrap();
    assert_eq!((menu.area.width, menu.area.height), (56, 15));
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn action_options_have_blank_spacing_that_does_not_trigger_actions() {
    let mut app = app();
    app.screen = Screen::Detail;
    let (terminal, hits) = render(&app, 80, 24);
    let rows = &hits.action_menu.as_ref().unwrap().rows;
    assert_eq!(rows.len(), 4);
    for pair in rows.windows(2) {
        assert_eq!(
            pair[1].0.y,
            pair[0].0.y + 2,
            "options have no blank row between them"
        );
        let gap = pair[0].0.y + 1;
        let line = text(&terminal)
            .lines()
            .nth(usize::from(gap))
            .unwrap()
            .to_string();
        for label in ["attach", "rename", "kill", "back"] {
            assert!(!line.contains(label));
        }
        assert_eq!(
            app.handle_mouse(
                mouse(MouseEventKind::Down(MouseButton::Left), pair[0].0.x, gap),
                &hits
            ),
            ManagerAction::None
        );
        assert_eq!(app.screen, Screen::Detail);
        assert_eq!(app.detail_selected, 0);
    }
}

#[test]
fn action_menu_key_help_fits_one_line() {
    let mut app = app();
    app.screen = Screen::Detail;
    for (width, prefix, close) in [
        (80, "↑↓/Tab", "Esc/q/點外 關閉"),
        (40, "↑↓ 選擇", "Esc 關閉"),
    ] {
        let (terminal, hits) = render(&app, width, 24);
        let menu = hits.action_menu.unwrap();
        let screen = text(&terminal);
        let footer: Vec<_> = screen
            .lines()
            .skip(usize::from(menu.area.y))
            .take(usize::from(menu.area.height))
            .filter(|line| line.contains(prefix))
            .collect();
        assert_eq!(footer.len(), 1);
        assert!(footer[0].contains("Enter"));
        assert!(
            footer[0].contains(close),
            "close instructions are not on the same visible row: {}",
            footer[0]
        );
        let back = menu.rows.last().unwrap().0;
        let help_row = screen
            .lines()
            .position(|line| line.contains(prefix))
            .unwrap() as u16;
        assert!(
            help_row >= back.y + 2,
            "help text has no gap after the last action"
        );
    }
}

#[test]
fn outside_click_only_dismisses_menu() {
    for target in ["session", "prompts", "divider"] {
        let mut app = app();
        app.split_percent = Some(40);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let selected = app.selected_id().cloned();
        let (_, hits) = render(&app, 80, 24);
        let (x, y) = match target {
            "session" => (1, 7),
            "prompts" => (4, 1),
            _ => (1, hits.panels.unwrap().divider.y),
        };
        assert_eq!(
            app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), x, y), &hits),
            ManagerAction::None
        );
        assert_eq!(
            app.screen,
            Screen::List,
            "outside {target} click did not dismiss menu"
        );
        assert_eq!(app.selected_id(), selected.as_ref());
        assert_eq!(app.split_percent, Some(40));
        assert!(!app.dragging_split);
        assert_eq!(app.preview, [preview_marker()]);
    }
}

#[test]
fn small_action_menu_keeps_selected_action_visible() {
    let mut app = app();
    app.sessions[0].name = "中文🙂超長名稱".repeat(15);
    app.screen = Screen::Detail;
    app.detail_selected = 3;
    for (width, height) in [(160, 40), (80, 24), (40, 12), (12, 8), (12, 5), (1, 1)] {
        let (terminal, hits) = render(&app, width, height);
        let menu = hits.action_menu.unwrap();
        let body = hits.panels.unwrap().body;
        assert!(menu.area.x >= body.x && menu.area.y >= body.y);
        assert!(menu.area.right() <= body.right() && menu.area.bottom() <= body.bottom());
        for (rect, _) in &menu.rows {
            assert!(rect.width > 0 && rect.height > 0);
            assert!(rect.x > menu.area.x && rect.right() < menu.area.right());
            assert!(rect.y > menu.area.y && rect.bottom() < menu.area.bottom());
        }
        if !menu.rows.is_empty() {
            assert!(menu.rows.iter().any(|(_, index)| *index == 3));
            if width >= 12 {
                assert!(text(&terminal).contains("back"));
            }
        }
    }
}

#[test]
fn menu_wheel_does_not_change_session_and_ignored_events_do_not_execute() {
    let mut app = app();
    app.screen = Screen::Detail;
    let (_, hits) = render(&app, 80, 24);
    let menu = hits.action_menu.as_ref().unwrap();
    let action = menu.rows[0].0;
    for kind in [
        MouseEventKind::Up(MouseButton::Left),
        MouseEventKind::Drag(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Moved,
    ] {
        assert_eq!(
            app.handle_mouse(mouse(kind, action.x, action.y), &hits),
            ManagerAction::None
        );
        assert_eq!(app.screen, Screen::Detail);
        assert_eq!(app.detail_selected, 0);
    }
    assert_eq!(
        app.handle_mouse(
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                menu.area.x,
                menu.area.y
            ),
            &hits
        ),
        ManagerAction::None
    );
    app.handle_mouse(mouse(MouseEventKind::ScrollDown, 0, 0), &hits);
    assert_eq!(app.detail_selected, 1);
    app.handle_mouse(mouse(MouseEventKind::ScrollUp, 0, 0), &hits);
    assert_eq!(app.detail_selected, 0);
    assert_eq!(app.selected_id().unwrap().as_str(), "$1");
    assert!(!app.handle_layout_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE), &hits));
    assert_eq!(app.split_percent, None);
}

#[test]
fn stale_menu_hit_cannot_retarget_another_session() {
    let mut app = app();
    app.screen = Screen::Detail;
    let (_, hits) = render(&app, 80, 24);
    let action = hits.action_menu.as_ref().unwrap().rows[0].0;
    let mut refreshed = app.sessions.clone();
    refreshed[0].name = "renamed".into();
    refreshed.reverse();
    app.replace_sessions(refreshed);
    assert_eq!(
        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), action.x, action.y),
            &hits
        ),
        ManagerAction::Attach
    );
    assert_eq!(app.selected_id().unwrap().as_str(), "$1");
    app.replace_sessions(vec![app.sessions[0].clone()]);
    assert_eq!(app.screen, Screen::List);
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.selected_id().unwrap().as_str(), "$2");
    assert_eq!(
        app.handle_mouse(
            mouse(MouseEventKind::Down(MouseButton::Left), action.x, action.y),
            &hits
        ),
        ManagerAction::None
    );
}

#[test]
fn ai_update_keeps_action_menu_target_and_selection() {
    let mut app = app();
    app.screen = Screen::Detail;
    app.detail_selected = 2;
    app.generation = 3;
    let id = app.selected_id().unwrap().clone();
    assert!(app.apply_ai(
        id.clone(),
        3,
        Ok(tmux_manager::ai::AiSummary {
            summary: "摘要已更新".into(),
            name: "updated".into()
        })
    ));
    let (terminal, _) = render(&app, 80, 24);
    assert!(text(&terminal).contains("摘要已更新"));
    assert_eq!(app.screen, Screen::Detail);
    assert_eq!(app.detail_selected, 2);
    assert_eq!(app.selected_id(), Some(&id));
}

#[tokio::test]
async fn attach_still_happens_when_the_prompt_dock_fails_to_start() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use tmux_manager::{app::actions::attach_with_dock, error::error};
    let attached = AtomicBool::new(false);
    let (supervisor, result) = attach_with_dock(
        async { Err::<(), _>(error("split-window failed")) },
        async {
            attached.store(true, Ordering::SeqCst);
            Ok(())
        },
    )
    .await;
    assert!(attached.load(Ordering::SeqCst));
    assert!(supervisor.is_none());
    let warning = result.unwrap().expect("dock failure should be reported");
    assert!(warning.contains("split-window failed"), "{warning}");
}

#[tokio::test]
async fn attach_with_a_running_dock_has_no_warning() {
    use tmux_manager::app::actions::attach_with_dock;
    let (supervisor, result) = attach_with_dock(async { Ok(7) }, async { Ok(()) }).await;
    assert_eq!(supervisor, Some(7));
    assert_eq!(result.unwrap(), None);
}

#[tokio::test]
async fn a_panicking_background_refresh_still_reports_back() {
    use tmux_manager::app::event::{AppEvent, spawn_event};
    // 沒有回報時 manager 的 io_busy 會一直為 true，之後不再刷新。
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    spawn_event(sender, async { panic!("snapshot parser bug") });
    match receiver.recv().await {
        Some(AppEvent::Error(message)) => assert!(message.contains("背景更新失敗"), "{message}"),
        _ => panic!("panicking refresh sent no error event"),
    }
}
