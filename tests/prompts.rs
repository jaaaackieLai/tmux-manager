use tmux_manager::prompts::{PromptDocument, PromptSlot, PromptStore};
#[test]
fn prompts_roundtrip_multiline_unicode_and_normalize_crlf() {
    let dir = tempfile::tempdir().unwrap();
    let store = PromptStore::new(dir.path().join("prompts.toml"));
    let snapshot = store.load().unwrap();
    let slot = PromptSlot::new(
        " 中文🙂 ",
        "中文\r\n'quote' \"雙引號\" \\path\n尾端\n",
        vec!["tag".into()],
        0,
    )
    .unwrap();
    let document = PromptDocument {
        schema_version: 1,
        slots: vec![slot],
    };
    store.commit(&snapshot.revision, &document).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.document.slots[0].title, "中文🙂");
    assert_eq!(
        loaded.document.slots[0].body,
        "中文\n'quote' \"雙引號\" \\path\n尾端\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(store.path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
#[test]
fn invalid_control_codes_and_body_limits() {
    for body in [
        " ",
        "",
        "hello\x1b[31m",
        "hello\0",
        "bare\rcarriage",
        "\x7f",
    ] {
        assert!(
            PromptSlot::new("title", body, vec![], 0).is_err(),
            "{body:?}"
        );
    }
    assert!(PromptSlot::new(" ", "body", vec![], 0).is_err());
    assert!(PromptSlot::new("title", &"a".repeat(65536), vec![], 0).is_ok());
    assert!(PromptSlot::new("title", &"a".repeat(65537), vec![], 0).is_err());
    assert!(PromptSlot::new("title", "a\tb\n", vec![], 0).is_ok());
}
#[test]
fn concurrent_edit_conflicts_and_corrupt_files_are_never_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompts.toml");
    let a = PromptStore::new(path.clone());
    let b = PromptStore::new(path.clone());
    let first = a.load().unwrap();
    let second = b.load().unwrap();
    let document = PromptDocument {
        schema_version: 1,
        slots: vec![PromptSlot::new("one", "body", vec![], 0).unwrap()],
    };
    a.commit(&first.revision, &document).unwrap();
    assert!(matches!(
        b.commit(&second.revision, &PromptDocument::default()),
        Err(tmux_manager::Error::Conflict)
    ));
    assert_eq!(a.load().unwrap().document.slots.len(), 1);
    std::fs::write(&path, "not valid [toml").unwrap();
    assert!(a.load().is_err());
    assert!(a.commit(&first.revision, &document).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "not valid [toml");
}
#[test]
fn invalid_schema_duplicate_identity_search_and_order() {
    let slot = PromptSlot::new("中文 title", "body", vec!["fix".into()], 9).unwrap();
    assert!(
        PromptDocument {
            schema_version: 2,
            slots: vec![]
        }
        .validate()
        .is_err()
    );
    assert!(
        PromptDocument {
            schema_version: 1,
            slots: vec![slot.clone(), slot.clone()]
        }
        .validate()
        .is_err()
    );
    let document = PromptDocument {
        schema_version: 1,
        slots: vec![slot, PromptSlot::new("other", "body", vec![], 1).unwrap()],
    };
    assert_eq!(document.search("中文")[0].title, "中文 title");
    assert_eq!(document.search("FIX").len(), 1);
    assert_eq!(document.search("")[0].title, "other");
}

#[test]
fn bracketed_paste_bare_carriage_returns_become_newlines() {
    use tmux_manager::prompts::editor::TextBuffer;
    let mut buffer = TextBuffer::new(String::new());
    buffer.insert("第一行\r第二行\r\n第三行\r").unwrap();
    assert_eq!(buffer.text, "第一行\n第二行\n第三行\n");
}
#[test]
fn editor_preserves_multiline_paste_and_requires_discard_confirmation() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use tmux_manager::prompts::editor::{EditorOutcome, PromptEditor};
    let slot = PromptSlot::new("title", "old", vec![], 0).unwrap();
    let mut editor = PromptEditor::edit(&slot);
    editor.focus = 2;
    editor
        .handle(Event::Paste("中文🙂\r\n尾端\n".into()))
        .unwrap();
    let result = editor
        .handle(Event::Key(KeyEvent::new(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
        )))
        .unwrap();
    let EditorOutcome::Save(saved) = result else {
        panic!("must save")
    };
    assert_eq!(saved.body, "old中文🙂\n尾端\n");
    assert_eq!(saved.id, slot.id);
    assert!(matches!(
        editor
            .handle(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .unwrap(),
        EditorOutcome::ConfirmDiscard
    ));
    assert!(editor.handle(Event::Paste("\x1b[2J".into())).is_err());
}
#[test]
fn mouse_hit_map_tracks_scrolled_and_filtered_uuid_and_ignores_release() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{Terminal, backend::TestBackend};
    use tmux_manager::prompts::ui::{PromptAction, PromptView};
    let dir = tempfile::tempdir().unwrap();
    let store = PromptStore::new(dir.path().join("prompts.toml"));
    let initial = store.load().unwrap();
    let document = PromptDocument {
        schema_version: 1,
        slots: (0..30)
            .map(|i| PromptSlot::new(&format!("prompt {i}"), "body", vec![], i).unwrap())
            .collect(),
    };
    let snapshot = store.commit(&initial.revision, &document).unwrap();
    let mut view = PromptView::new(snapshot, None);
    view.selected = Some(document.slots[20].id);
    let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    terminal
        .draw(|f| {
            view.render(f);
        })
        .unwrap();
    let (rect, id) = *view.hits.last().unwrap();
    assert!(view.offset > 0);
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x,
        row: rect.y,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(view.handle_mouse(mouse), Some(PromptAction::Paste(id)));
    assert_eq!(
        view.handle_mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            ..mouse
        }),
        None
    );
    assert_eq!(
        view.handle_mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            ..mouse
        }),
        None
    );
    view.query = "prompt 29".into();
    view.selected = Some(document.slots[29].id);
    terminal
        .draw(|f| {
            view.render(f);
        })
        .unwrap();
    assert_eq!(view.hits[0].1, document.slots[29].id);
}
#[test]
fn saving_conflict_keeps_editor_draft_and_document_intact() {
    use tmux_manager::prompts::{editor::PromptEditor, ui::PromptView};
    let dir = tempfile::tempdir().unwrap();
    let store = PromptStore::new(dir.path().join("prompts.toml"));
    let snapshot = store.load().unwrap();
    let mut view = PromptView::new(snapshot, None);
    view.editor = Some(PromptEditor::create(0));
    let external = PromptDocument {
        schema_version: 1,
        slots: vec![PromptSlot::new("external", "body", vec![], 0).unwrap()],
    };
    store.commit(&view.snapshot.revision, &external).unwrap();
    let draft = PromptSlot::new("draft", "my draft", vec![], 1).unwrap();
    assert!(view.save(&store, draft).is_err());
    assert!(view.editor.is_some());
    assert_eq!(store.load().unwrap().document.slots[0].title, "external");
}
fn rendered_text(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}
#[test]
fn narrow_editor_keeps_active_body_and_long_line_cursor_text_visible() {
    use ratatui::{Terminal, backend::TestBackend};
    use tmux_manager::prompts::{editor::PromptEditor, ui::PromptView};
    let dir = tempfile::tempdir().unwrap();
    let snapshot = PromptStore::new(dir.path().join("prompts.toml"))
        .load()
        .unwrap();
    for (width, height) in [(80, 24), (40, 12)] {
        let mut view = PromptView::new(snapshot.clone(), None);
        let body = format!("{}VISIBLE_CURSOR", "中文".repeat(60));
        let slot = PromptSlot::new("title", &body, vec![], 0).unwrap();
        let mut editor = PromptEditor::edit(&slot);
        editor.focus = 2;
        view.editor = Some(editor);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                view.render(f);
            })
            .unwrap();
        assert!(
            rendered_text(&terminal).contains("VISIBLE_CURSOR"),
            "cursor content hidden at {width}x{height}"
        );
    }
}
#[test]
fn failure_buffer_name_and_confirmation_remain_visible_in_narrow_ui() {
    use ratatui::{Terminal, backend::TestBackend};
    use tmux_manager::prompts::ui::PromptView;
    let dir = tempfile::tempdir().unwrap();
    let snapshot = PromptStore::new(dir.path().join("prompts.toml"))
        .load()
        .unwrap();
    let buffer = "tmux-manager-5b7d3df5-9445-4206-b3c8-941697ed995c";
    for (width, height) in [(80, 24), (40, 12)] {
        let mut view = PromptView::new(
            snapshot.clone(),
            Some("%12 · session/0:work.1 長目標描述".into()),
        );
        view.status = format!(
            "{} buffer: {buffer}",
            "無法確認目標 bracketed paste 能力，尚未貼上。".repeat(3)
        );
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                view.render(f);
            })
            .unwrap();
        assert!(
            rendered_text(&terminal).replace(' ', "").contains(buffer),
            "buffer name hidden at {width}x{height}"
        );
        view.status = "放棄未儲存草稿？[y/N]".into();
        terminal
            .draw(|f| {
                view.render(f);
            })
            .unwrap();
        assert!(rendered_text(&terminal).contains("[y/N]"));
    }
}
#[test]
#[cfg(unix)]
fn failed_atomic_write_preserves_original_prompt_document() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let store = PromptStore::new(dir.path().join("prompts.toml"));
    let original = store.load().unwrap();
    let document = PromptDocument {
        schema_version: 1,
        slots: vec![PromptSlot::new("original", "body", vec![], 0).unwrap()],
    };
    let saved = store.commit(&original.revision, &document).unwrap();
    let before = std::fs::read(store.path()).unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = store.commit(&saved.revision, &PromptDocument::default());
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
}

fn store_with_one_slot() -> (tempfile::TempDir, PromptStore, uuid::Uuid) {
    let dir = tempfile::tempdir().unwrap();
    let store = PromptStore::new(dir.path().join("prompts.toml"));
    let snapshot = store.load().unwrap();
    let mut document = snapshot.document.clone();
    let slot = PromptSlot::new("review", "old body", vec![], 0).unwrap();
    let id = slot.id;
    document.slots.push(slot);
    store.commit(&snapshot.revision, &document).unwrap();
    (dir, store, id)
}

#[test]
fn pasting_reads_the_current_file_instead_of_a_stale_snapshot() {
    let (_dir, store, id) = store_with_one_slot();
    let mut stale = store.load().unwrap();
    // 另一個視窗刪除了這個 prompt；舊 snapshot 仍有它。
    let mut document = stale.document.clone();
    document.slots.clear();
    store.commit(&stale.revision, &document).unwrap();
    let error = store.current_slot(&mut stale, id).unwrap_err().to_string();
    assert!(error.contains("已被移除"), "{error}");
    assert!(
        stale.document.slots.is_empty(),
        "snapshot should be refreshed"
    );
}

#[test]
fn a_recovered_reload_clears_its_error() {
    let (_dir, store, _id) = store_with_one_slot();
    let mut snapshot = store.load().unwrap();
    let mut error = None;
    std::fs::write(store.path(), "not = [valid").unwrap();
    assert!(store.reload_into(&mut snapshot, &mut error));
    assert!(error.is_some());
    assert!(
        !store.reload_into(&mut snapshot, &mut error),
        "same error must not redraw"
    );
    // 改回與 snapshot 相同的內容時沒有新資料，但錯誤仍要清除。
    let document = snapshot.document.clone();
    std::fs::write(store.path(), toml::to_string(&document).unwrap()).unwrap();
    assert!(store.reload_into(&mut snapshot, &mut error));
    assert_eq!(error, None);
}
