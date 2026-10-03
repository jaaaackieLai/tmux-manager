use super::{PromptSnapshot, editor::PromptEditor};
use crate::ui::hit_test;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use uuid::Uuid;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptAction {
    Paste(Uuid),
    Create,
    Edit(Uuid),
    Delete(Uuid),
    Move(Uuid, i32),
    Cancel,
    Reload,
}
pub struct PromptView {
    pub snapshot: PromptSnapshot,
    pub target: Option<String>,
    pub selected: Option<Uuid>,
    pub query: String,
    pub searching: bool,
    pub offset: usize,
    pub hits: Vec<(Rect, Uuid)>,
    pub editor: Option<PromptEditor>,
    pub status: String,
    /// 背景重讀失敗的訊息；存在時取代 `status` 顯示，恢復後清除。
    pub reload_error: Option<String>,
    pub status_scroll: u16,
    last_status: String,
}
impl PromptView {
    pub fn new(snapshot: PromptSnapshot, target: Option<String>) -> Self {
        let selected = snapshot.document.search("").first().map(|s| s.id);
        Self {
            snapshot,
            target,
            selected,
            query: String::new(),
            searching: false,
            offset: 0,
            hits: Vec::new(),
            editor: None,
            status: String::new(),
            reload_error: None,
            status_scroll: 0,
            last_status: String::new(),
        }
    }
    pub fn navigate(&mut self, delta: i32) {
        let slots = self.snapshot.document.search(&self.query);
        let index = self
            .selected
            .and_then(|id| slots.iter().position(|s| s.id == id))
            .unwrap_or(0);
        let next = (index as i64 + i64::from(delta)).clamp(0, slots.len().saturating_sub(1) as i64)
            as usize;
        self.selected = slots.get(next).map(|s| s.id);
    }
    pub fn handle_mouse(&mut self, event: MouseEvent) -> Option<PromptAction> {
        match event.kind {
            MouseEventKind::ScrollUp => {
                self.navigate(-1);
                None
            }
            MouseEventKind::ScrollDown => {
                self.navigate(1);
                None
            }
            _ => hit_test::click(&self.hits, event).copied().map(|id| {
                self.selected = Some(id);
                PromptAction::Paste(id)
            }),
        }
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<PromptAction> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(PromptAction::Cancel);
        }
        if self.searching {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.query.pop();
                }
                KeyCode::Char(c) => self.query.push(c),
                _ => (),
            }
            self.selected = self
                .snapshot
                .document
                .search(&self.query)
                .first()
                .map(|s| s.id);
            self.offset = 0;
            return None;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Some(PromptAction::Cancel),
            KeyCode::Up => {
                self.navigate(-1);
                None
            }
            KeyCode::Down | KeyCode::Tab => {
                self.navigate(1);
                None
            }
            KeyCode::Enter => self.selected.map(PromptAction::Paste),
            KeyCode::Char('n') => Some(PromptAction::Create),
            KeyCode::Char('e') => self.selected.map(PromptAction::Edit),
            KeyCode::Char('d') => self.selected.map(PromptAction::Delete),
            KeyCode::Char('K') => self.selected.map(|id| PromptAction::Move(id, -1)),
            KeyCode::Char('J') => self.selected.map(|id| PromptAction::Move(id, 1)),
            KeyCode::Char('/') => {
                self.searching = true;
                None
            }
            KeyCode::PageDown => {
                self.status_scroll = self.status_scroll.saturating_add(2);
                None
            }
            KeyCode::PageUp => {
                self.status_scroll = self.status_scroll.saturating_sub(2);
                None
            }
            KeyCode::Char('f') => Some(PromptAction::Reload),
            _ => None,
        }
    }
    pub fn render(&mut self, frame: &mut Frame) {
        self.hits.clear();
        let status = self
            .reload_error
            .clone()
            .unwrap_or_else(|| self.status.clone());
        if self.last_status != status {
            self.status_scroll = 0;
            self.last_status = status.clone();
        }
        let area = frame.area();
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(if status.is_empty() { 2 } else { 6 }),
            ])
            .split(area);
        frame.render_widget(
            Paragraph::new(format!(
                "Prompt Slots · / 搜尋: {}{}",
                self.query,
                if self.searching { "▏" } else { "" }
            ))
            .style(Style::default().fg(Color::Cyan)),
            parts[0],
        );
        if let Some(editor) = &self.editor {
            render_editor(frame, parts[1], editor);
        } else {
            let body = Layout::default()
                .direction(if area.width >= 70 {
                    Direction::Horizontal
                } else {
                    Direction::Vertical
                })
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(parts[1]);
            let block = Block::default()
                .borders(Borders::ALL)
                .title("點選貼上 · n 新增 · e 編輯");
            let inner = block.inner(body[0]);
            frame.render_widget(block, body[0]);
            let slots = self.snapshot.document.search(&self.query);
            if !slots.iter().any(|s| Some(s.id) == self.selected) {
                self.selected = slots.first().map(|s| s.id);
            }
            let selected_index = self
                .selected
                .and_then(|id| slots.iter().position(|s| s.id == id))
                .unwrap_or(0);
            let count = usize::from(inner.height);
            if count > 0 {
                if selected_index < self.offset {
                    self.offset = selected_index;
                }
                if selected_index >= self.offset + count {
                    self.offset = selected_index + 1 - count;
                }
                self.offset = self.offset.min(slots.len().saturating_sub(1));
                for (row, slot) in slots.iter().skip(self.offset).take(count).enumerate() {
                    let rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
                    let selected = Some(slot.id) == self.selected;
                    frame.render_widget(
                        Paragraph::new(format!(
                            "{} {}",
                            if selected { ">" } else { " " },
                            slot.title
                        ))
                        .style(Style::default().fg(if selected {
                            Color::Yellow
                        } else {
                            Color::White
                        })),
                        rect,
                    );
                    self.hits.push((rect, slot.id));
                }
            }
            let preview = slots
                .iter()
                .find(|s| Some(s.id) == self.selected)
                .map(|s| s.body.as_str())
                .unwrap_or("尚無 prompt；按 n 新增");
            frame.render_widget(
                Paragraph::new(preview)
                    .block(Block::default().borders(Borders::ALL).title("Preview"))
                    .wrap(Wrap { trim: false }),
                body[1],
            );
        }
        let footer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(parts[2]);
        let target = self
            .target
            .as_deref()
            .unwrap_or("未指定（管理模式，禁止貼上）");
        let keys = if self.editor.is_some() {
            "Tab:欄位 Ctrl-S:存 Esc:取消 PgUp/Dn:訊息"
        } else {
            "Enter:貼 n/e/d /搜尋 J/K PgDn Esc"
        };
        let message = visible_status(&status);
        frame.render_widget(
            Paragraph::new(message)
                .wrap(Wrap { trim: false })
                .scroll((self.status_scroll, 0))
                .style(Style::default().fg(Color::Yellow)),
            footer[0],
        );
        frame.render_widget(Paragraph::new(format!("目標：{target}")), footer[1]);
        frame.render_widget(Paragraph::new(keys), footer[2]);
    }
}
fn visible_status(status: &str) -> String {
    if let Some(index) = status.find("tmux-manager-") {
        let buffer: String = status[index..].chars().take(49).collect();
        format!("buffer: {buffer}\n{status}")
    } else {
        status.to_string()
    }
}
fn render_editor(frame: &mut Frame, area: Rect, editor: &PromptEditor) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(area);
    for (index, title) in ["標題", "標籤（逗號分隔）", "內容（多行）"]
        .iter()
        .enumerate()
    {
        if area.height < 9 && index != editor.focus {
            continue;
        }
        let field_area = if area.height < 9 { area } else { areas[index] };
        let field = &editor.fields[index];
        let block = Block::default()
            .borders(Borders::ALL)
            .title(*title)
            .border_style(Style::default().fg(if index == editor.focus {
                Color::Yellow
            } else {
                Color::Gray
            }));
        let inner = block.inner(field_area);
        let line = field.text[..field.cursor]
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
        let top = line.saturating_sub(usize::from(inner.height).saturating_sub(1));
        let prefix = field.text[..field.cursor].rsplit('\n').next().unwrap_or("");
        let width = unicode_width::UnicodeWidthStr::width(prefix);
        let left = width.saturating_sub(usize::from(inner.width).saturating_sub(1));
        frame.render_widget(
            Paragraph::new(field.text.as_str()).block(block).scroll((
                top.min(u16::MAX as usize) as u16,
                left.min(u16::MAX as usize) as u16,
            )),
            field_area,
        );
        if index == editor.focus && inner.width > 0 && inner.height > 0 {
            frame.set_cursor_position((
                inner.x + (width - left).min(usize::from(inner.width - 1)) as u16,
                inner.y + (line - top).min(usize::from(inner.height - 1)) as u16,
            ));
        }
    }
}
