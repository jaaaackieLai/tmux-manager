use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tmux_manager::{
    app::{AppState, ManagerAction, Screen},
    tmux::{Session, SessionId},
};
fn sessions() -> Vec<Session> {
    vec![
        Session {
            id: SessionId::parse("$1").unwrap(),
            name: "中文🙂".into(),
            windows: 2,
            created: 123,
        },
        Session {
            id: SessionId::parse("$2").unwrap(),
            name: "other".into(),
            windows: 1,
            created: 123,
        },
    ]
}
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
#[test]
fn list_detail_navigation_and_shortcuts() {
    let mut app = AppState::default();
    app.replace_sessions(sessions());
    for _ in 0..2 {
        assert_eq!(app.handle_key(key(KeyCode::Right)), ManagerAction::None);
        assert_eq!(app.screen, Screen::List);
    }
    assert_eq!(app.handle_key(key(KeyCode::Down)), ManagerAction::None);
    assert_eq!(app.selected_id().unwrap().as_str(), "$2");
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::Detail);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        ManagerAction::Prompts
    );
    assert_eq!(
        app.handle_key(key(KeyCode::Char('a'))),
        ManagerAction::Attach
    );
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.screen, Screen::List);
    assert_eq!(app.handle_key(key(KeyCode::Char('k'))), ManagerAction::None);
}

#[test]
fn list_can_open_prompt_management_without_sessions_or_changing_selection() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        ManagerAction::Prompts
    );
    assert_eq!(app.screen, Screen::List);
    app.replace_sessions(sessions());
    app.handle_key(key(KeyCode::Down));
    assert_eq!(
        app.handle_key(key(KeyCode::Char('p'))),
        ManagerAction::Prompts
    );
    assert_eq!(app.selected_id().unwrap().as_str(), "$2");
    assert_eq!(app.screen, Screen::List);
}

#[test]
fn manager_prompt_entry_is_clickable_in_empty_list_and_preserves_selection() {
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};
    for available in [vec![], sessions()] {
        let mut app = AppState::default();
        app.replace_sessions(available);
        app.selected = app.sessions.len().saturating_sub(1);
        let selected = app.selected_id().cloned();
        for (width, height) in [(80, 24), (40, 12)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
            terminal
                .draw(|frame| {
                    hits = tmux_manager::ui::manager::render(frame, &app);
                })
                .unwrap();
            assert_eq!(
                app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 4, 1), &hits),
                ManagerAction::Prompts
            );
            assert_eq!(app.screen, Screen::List);
            assert_eq!(app.selected_id(), selected.as_ref());
            assert_eq!(
                app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 4, 1), &hits),
                ManagerAction::None
            );
        }
    }
}
#[test]
fn selection_tracks_stable_ids_and_survives_rename_and_removal() {
    let mut app = AppState::default();
    app.replace_sessions(sessions());
    app.handle_key(key(KeyCode::Down));
    let mut refreshed = sessions();
    refreshed[1].name = "renamed".into();
    refreshed.reverse();
    app.replace_sessions(refreshed);
    assert_eq!(app.selected_id().unwrap().as_str(), "$2");
    app.replace_sessions(vec![]);
    assert!(app.selected_id().is_none());
    assert_eq!(app.handle_key(key(KeyCode::Enter)), ManagerAction::None);
    assert_eq!(app.handle_key(key(KeyCode::Char('n'))), ManagerAction::New);
}
#[test]
fn stale_ai_results_are_discarded_but_rename_keeps_result() {
    let mut app = AppState::default();
    app.replace_sessions(sessions());
    let id = SessionId::parse("$1").unwrap();
    app.generation = 2;
    assert!(!app.apply_ai(
        id.clone(),
        1,
        Ok(tmux_manager::ai::AiSummary {
            summary: "stale".into(),
            name: "old".into()
        })
    ));
    assert!(app.apply_ai(
        id.clone(),
        2,
        Ok(tmux_manager::ai::AiSummary {
            summary: "開發中".into(),
            name: "dev".into()
        })
    ));
    let mut next = sessions();
    next[0].name = "changed".into();
    app.replace_sessions(next);
    assert_eq!(app.ai[&id].as_ref().unwrap().summary, "開發中");
}
#[test]
fn manager_renders_chinese_and_small_terminals_without_overflow() {
    use ratatui::{Terminal, backend::TestBackend};
    let mut app = AppState::default();
    app.replace_sessions(sessions());
    for (width, height) in [(80, 24), (40, 12), (12, 5), (1, 1)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                tmux_manager::ui::manager::render(f, &app);
            })
            .unwrap();
        assert_eq!(terminal.backend().buffer().area.width, width);
    }
}

fn mouse(
    kind: crossterm::event::MouseEventKind,
    column: u16,
    row: u16,
) -> crossterm::event::MouseEvent {
    crossterm::event::MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn mouse_uses_visible_scrolled_session_rows_and_ignores_border_release_and_drag() {
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};
    let mut app = AppState::default();
    app.replace_sessions(
        (0..12)
            .map(|index| Session {
                id: SessionId::parse(&format!("${index}")).unwrap(),
                name: format!("session-{index}"),
                windows: 1,
                created: 123,
            })
            .collect(),
    );
    app.selected = 11;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
    terminal
        .draw(|frame| {
            hits = tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    let first_row = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .skip(5 * 80)
        .take(80)
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(first_row.contains("session-9"), "{first_row}");
    for (kind, y) in [
        (MouseEventKind::Down(MouseButton::Left), 2),
        (MouseEventKind::Down(MouseButton::Left), 3),
        (MouseEventKind::Down(MouseButton::Left), 6),
        (MouseEventKind::Up(MouseButton::Left), 5),
        (MouseEventKind::Drag(MouseButton::Left), 5),
        (MouseEventKind::Down(MouseButton::Right), 5),
    ] {
        assert_eq!(
            app.handle_mouse(mouse(kind, 5, y), &hits),
            ManagerAction::None
        );
        assert_eq!(app.screen, Screen::List);
        assert_eq!(app.selected_id().unwrap().as_str(), "$11");
    }
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 5, 5), &hits);
    assert_eq!(app.screen, Screen::Detail);
    assert_eq!(app.selected_id().unwrap().as_str(), "$9");
}

#[test]
fn session_mouse_target_survives_refresh_reorder_and_rejects_removed_identity() {
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};
    let mut app = AppState::default();
    app.replace_sessions(sessions());
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
    terminal
        .draw(|frame| {
            hits = tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    let mut reordered = sessions();
    reordered.reverse();
    app.replace_sessions(reordered);
    let click = mouse(MouseEventKind::Down(MouseButton::Left), 5, 7);
    app.handle_mouse(click, &hits);
    assert_eq!(app.selected_id().unwrap().as_str(), "$2");
    assert_eq!(app.screen, Screen::Detail);
    app.screen = Screen::List;
    app.replace_sessions(vec![sessions()[0].clone()]);
    app.handle_mouse(click, &hits);
    assert_eq!(app.screen, Screen::List);
    assert_eq!(app.selected_id().unwrap().as_str(), "$1");
}

#[test]
fn detail_mouse_clicks_follow_keyboard_actions_and_wheel_only_selects() {
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};
    for (index, expected) in [
        (0, ManagerAction::Attach),
        (1, ManagerAction::Rename),
        (2, ManagerAction::Kill),
        (3, ManagerAction::None),
    ] {
        let mut app = AppState::default();
        app.replace_sessions(sessions());
        app.screen = Screen::Detail;
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
        terminal
            .draw(|frame| {
                hits = tmux_manager::ui::manager::render(frame, &app);
            })
            .unwrap();
        assert_eq!(
            app.handle_mouse(mouse(MouseEventKind::ScrollDown, 5, 6), &hits),
            ManagerAction::None
        );
        assert_eq!(app.detail_selected, 1);
        let rect = hits.action_menu.as_ref().unwrap().rows[index as usize].0;
        assert_eq!(
            app.handle_mouse(
                mouse(MouseEventKind::Down(MouseButton::Left), rect.x, rect.y),
                &hits
            ),
            expected
        );
        assert_eq!(
            app.screen,
            if index == 3 {
                Screen::List
            } else {
                Screen::Detail
            }
        );
    }
}
#[test]
fn vanished_detail_session_returns_to_list_instead_of_retargeting() {
    let mut app = AppState::default();
    app.replace_sessions(sessions());
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.screen, Screen::Detail);
    app.replace_sessions(sessions());
    assert_eq!(app.screen, Screen::Detail, "同一 session 仍在時留在詳細頁");
    app.replace_sessions(sessions()[..1].to_vec());
    assert_eq!(app.screen, Screen::List);
    assert_eq!(app.handle_key(key(KeyCode::Char('k'))), ManagerAction::None);
}

#[test]
fn refresh_errors_are_shown_beside_other_messages_and_cleared_on_recovery() {
    use tmux_manager::app::AppState;
    let mut app = AppState {
        status: "已取消".into(),
        ..AppState::default()
    };
    assert!(app.show_refresh_error("tmux 指令超時（2 秒）".into()));
    // 每次 poll 都可能回報同一個錯誤；內容沒變就不重畫，也不覆蓋使用者看到的訊息。
    assert!(!app.show_refresh_error("tmux 指令超時（2 秒）".into()));
    assert_eq!(app.status, "已取消");
    let line = app.status_line();
    assert!(
        line.contains("已取消") && line.contains("tmux 指令超時"),
        "{line}"
    );
    assert!(app.refresh_succeeded());
    assert_eq!(app.status_line(), "已取消");
    assert!(!app.refresh_succeeded());
}
