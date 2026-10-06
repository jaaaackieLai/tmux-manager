mod support;
use ratatui::{Terminal, backend::TestBackend};
use tmux_manager::{
    app::AppState,
    tmux::{Session, SessionId},
};

fn app_with(count: usize) -> AppState {
    let mut app = AppState::default();
    app.replace_sessions(
        (0..count)
            .map(|index| Session {
                id: SessionId::parse(&format!("${index}")).unwrap(),
                name: format!("session-{index}"),
                windows: 1,
                created: 1,
            })
            .collect(),
    );
    app
}

use support::{preview::pane, render::row_text};

#[test]
fn small_session_list_uses_content_height_and_releases_room_for_preview() {
    let app = app_with(5);
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    terminal
        .draw(|frame| {
            tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    let preview_row = (0..40)
        .find(|y| row_text(&terminal, *y).contains("Preview"))
        .unwrap();
    assert!(
        preview_row <= 18,
        "five sessions leave an oversized list: preview starts at {preview_row}"
    );
}

#[test]
fn session_rows_have_spacing_aligned_columns_and_a_visible_selection_background() {
    let mut app = app_with(2);
    app.sessions[0].name = "中文🙂".into();
    app.sessions[0].windows = 2;
    app.sessions[1].name = "longer-name".into();
    app.sessions[1].windows = 11;
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    terminal
        .draw(|frame| {
            tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    let first = (0..40)
        .find(|y| row_text(&terminal, *y).contains("中文🙂"))
        .unwrap();
    let second = (0..40)
        .find(|y| row_text(&terminal, *y).contains("longer-name"))
        .unwrap();
    assert!(second >= first + 2, "session rows are crowded");
    let first_line = row_text(&terminal, first);
    let second_line = row_text(&terminal, second);
    let first_x =
        unicode_width::UnicodeWidthStr::width(first_line.split("2 windows").next().unwrap());
    let second_x =
        unicode_width::UnicodeWidthStr::width(second_line.split("11 windows").next().unwrap());
    assert_eq!(first_x, second_x, "window count columns do not align");
    let buffer = terminal.backend().buffer();
    assert_ne!(
        buffer[(5, first)].bg,
        buffer[(5, second)].bg,
        "selected row has no background highlight"
    );
}

#[test]
fn short_preview_keeps_the_latest_output_visible() {
    let mut app = app_with(1);
    let output: Vec<_> = (1..=15).map(|i| format!("OUTPUT-{i:02}")).collect();
    app.preview = vec![pane(0, 0, 80, true, "zsh", &output.join("\n"))];
    let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    terminal
        .draw(|frame| {
            tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    assert!(
        (0..12).any(|y| row_text(&terminal, y).contains("OUTPUT-15")),
        "latest output is hidden by the small preview:\n{}",
        (0..12)
            .map(|y| row_text(&terminal, y))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(!(0..12).any(|y| row_text(&terminal, y).contains("OUTPUT-01")));
}

#[test]
fn preview_shows_every_pane_of_the_selected_window_side_by_side() {
    let mut app = app_with(1);
    app.preview = vec![
        pane(0, 0, 40, true, "claude", "LEFT-OUTPUT"),
        pane(1, 41, 39, false, "zsh", "RIGHT-OUTPUT"),
    ];
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| {
            tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    let title = (0..24)
        .find(|y| row_text(&terminal, *y).contains("Preview · session-0"))
        .expect("preview title names the selected session");
    let row = row_text(&terminal, title + 1);
    assert!(row.contains("0: claude") && row.contains("1: zsh"), "{row}");
    let row = row_text(&terminal, title + 2);
    assert!(
        row.contains("LEFT-OUTPUT") && row.contains("RIGHT-OUTPUT"),
        "{row}"
    );
}

#[test]
fn dragging_only_the_divider_resizes_panels_without_opening_a_session() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use tmux_manager::app::Screen;
    let mut app = app_with(5);
    let original = app.selected_id().cloned();
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
    terminal
        .draw(|frame| {
            hits = tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    for (kind, row) in [
        (MouseEventKind::Down(MouseButton::Left), 16),
        (MouseEventKind::Drag(MouseButton::Left), 24),
        (MouseEventKind::Up(MouseButton::Left), 24),
    ] {
        app.handle_mouse(
            MouseEvent {
                kind,
                column: 50,
                row,
                modifiers: KeyModifiers::NONE,
            },
            &hits,
        );
    }
    terminal
        .draw(|frame| {
            tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    assert!(
        row_text(&terminal, 25).contains("Preview"),
        "divider drag did not move the preview to the requested row"
    );
    assert_eq!(app.screen, Screen::List);
    assert_eq!(app.selected_id(), original.as_ref());
}

#[test]
fn keyboard_resizing_starts_from_actual_layout_and_can_restore_auto() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = app_with(5);
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
    terminal
        .draw(|frame| {
            hits = tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    assert!(app.handle_layout_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE), &hits));
    terminal
        .draw(|frame| {
            hits = tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    assert!(
        row_text(&terminal, 18).contains("Preview"),
        "keyboard resize did not start at the automatic divider"
    );
    assert!(app.handle_layout_key(
        KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::NONE),
        &hits
    ));
    terminal
        .draw(|frame| {
            tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    assert!(row_text(&terminal, 17).contains("Preview"));
    app.screen = tmux_manager::app::Screen::Detail;
    assert!(!app.handle_layout_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE), &hits));
}

#[test]
fn narrow_layout_keeps_ai_on_its_own_line_and_bounds_manual_panels_after_resize() {
    let mut app = app_with(1);
    app.sessions[0].name = "中文🙂超長名稱測試非常長的Session".into();
    app.ai.insert(
        app.sessions[0].id.clone(),
        Ok(tmux_manager::ai::AiSummary {
            summary: "正在整理摘要".into(),
            name: "dev".into(),
        }),
    );
    let mut terminal = Terminal::new(TestBackend::new(40, 24)).unwrap();
    let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
    terminal
        .draw(|frame| {
            hits = tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    let title_row = (0..24)
        .find(|y| row_text(&terminal, *y).contains("中文🙂"))
        .unwrap();
    assert!(row_text(&terminal, title_row).contains('…'));
    assert!(row_text(&terminal, title_row + 1).contains("正在整理摘要"));
    app.split_percent = Some(90);
    for (width, height) in [(40, 12), (80, 24), (160, 40), (1, 1)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| {
                hits = tmux_manager::ui::manager::render(frame, &app);
            })
            .unwrap();
        let panels = hits.panels.unwrap();
        assert!(panels.preview.bottom() <= height);
        assert!(panels.sessions.bottom() <= panels.divider.y);
        assert!(panels.divider.bottom() <= panels.preview.y);
        if panels.body.height >= 7 {
            assert!(panels.sessions.height >= 3 && panels.preview.height >= 3);
        }
    }
}

#[test]
fn keyboard_resize_moves_at_least_one_line_in_a_small_viewport() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = app_with(1);
    app.split_percent = Some(56);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
    terminal
        .draw(|frame| {
            hits = tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    let before = hits.panels.unwrap().sessions.height;
    app.handle_layout_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE), &hits);
    terminal
        .draw(|frame| {
            hits = tmux_manager::ui::manager::render(frame, &app);
        })
        .unwrap();
    assert!(
        hits.panels.unwrap().sessions.height > before,
        "5 percentage points rounded down to no movement"
    );
}

#[test]
fn opening_prompts_by_keyboard_cancels_an_unfinished_divider_drag() {
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use tmux_manager::app::ManagerAction;
    let mut app = app_with(5);
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
    terminal
        .draw(|frame| hits = tmux_manager::ui::manager::render(frame, &app))
        .unwrap();
    let event = |kind, row| MouseEvent {
        kind,
        column: 50,
        row,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(
        event(
            MouseEventKind::Down(MouseButton::Left),
            hits.panels.unwrap().divider.y,
        ),
        &hits,
    );
    assert!(app.dragging_split);
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
        ManagerAction::Prompts
    );
    assert!(
        !app.dragging_split,
        "prompt mode left the divider drag active"
    );
    app.handle_mouse(event(MouseEventKind::Drag(MouseButton::Left), 25), &hits);
    assert_eq!(
        app.split_percent, None,
        "ordinary drag resized the list after prompts"
    );
}

#[test]
fn a_new_mouse_press_outside_the_divider_cancels_any_old_drag() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let mut app = app_with(5);
    let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
    let mut hits = tmux_manager::ui::manager_mouse::ManagerHitMap::default();
    terminal
        .draw(|frame| hits = tmux_manager::ui::manager::render(frame, &app))
        .unwrap();
    let event = |kind, row| MouseEvent {
        kind,
        column: 50,
        row,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(
        event(
            MouseEventKind::Down(MouseButton::Left),
            hits.panels.unwrap().divider.y,
        ),
        &hits,
    );
    assert!(app.dragging_split);
    app.handle_mouse(event(MouseEventKind::Down(MouseButton::Left), 25), &hits);
    assert!(
        !app.dragging_split,
        "new press retained a stale divider drag"
    );
    app.handle_mouse(event(MouseEventKind::Drag(MouseButton::Left), 26), &hits);
    assert_eq!(app.split_percent, None);
}
