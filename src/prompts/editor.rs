use super::{
    PromptSlot,
    model::{MAX_BODY_BYTES, is_forbidden_control},
};
use crate::{Result, error::error};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;
#[derive(Clone, Debug)]
pub struct TextBuffer {
    pub text: String,
    pub cursor: usize,
}
impl TextBuffer {
    pub fn new(text: String) -> Self {
        let cursor = text.len();
        Self { text, cursor }
    }
    pub fn insert(&mut self, text: &str) -> Result<()> {
        // 終端的 bracketed paste 多以單獨 CR 表示換行。
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if text.chars().any(is_forbidden_control) {
            return Err(error("輸入含不允許的控制字元"));
        }
        if self.text.len() + text.len() > MAX_BODY_BYTES {
            return Err(error("輸入上限 64 KiB"));
        }
        self.text.insert_str(self.cursor, &text);
        self.cursor += text.len();
        Ok(())
    }
    /// `at` 所在行的起點（byte offset）。
    fn line_start(&self, at: usize) -> usize {
        self.text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0)
    }
    fn previous(&self) -> usize {
        self.text[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
    fn next(&self) -> usize {
        self.cursor
            + self.text[self.cursor..]
                .graphemes(true)
                .next()
                .map(str::len)
                .unwrap_or(0)
    }
    pub fn key(&mut self, code: KeyCode, multiline: bool) -> Result<()> {
        match code {
            KeyCode::Char(c) => self.insert(&c.to_string())?,
            KeyCode::Enter if multiline => self.insert("\n")?,
            KeyCode::Left => self.cursor = self.previous(),
            KeyCode::Right => self.cursor = self.next(),
            KeyCode::Home => self.cursor = self.line_start(self.cursor),
            KeyCode::End => {
                self.cursor += self.text[self.cursor..]
                    .find('\n')
                    .unwrap_or(self.text.len() - self.cursor)
            }
            KeyCode::Backspace if self.cursor > 0 => {
                let previous = self.previous();
                self.text.replace_range(previous..self.cursor, "");
                self.cursor = previous;
            }
            KeyCode::Delete if self.cursor < self.text.len() => {
                let next = self.next();
                self.text.replace_range(self.cursor..next, "");
            }
            KeyCode::Up | KeyCode::Down => {
                let start = self.line_start(self.cursor);
                let column = self.text[start..self.cursor].graphemes(true).count();
                let destination = if code == KeyCode::Up {
                    if start == 0 {
                        None
                    } else {
                        Some(self.line_start(start - 1))
                    }
                } else {
                    self.text[self.cursor..]
                        .find('\n')
                        .map(|i| self.cursor + i + 1)
                };
                if let Some(destination) = destination {
                    let line = self.text[destination..].split('\n').next().unwrap_or("");
                    self.cursor = destination
                        + line
                            .graphemes(true)
                            .take(column)
                            .map(str::len)
                            .sum::<usize>();
                }
            }
            _ => (),
        }
        Ok(())
    }
}
pub struct PromptEditor {
    pub fields: [TextBuffer; 3],
    pub focus: usize,
    original: PromptSlot,
}
#[derive(Debug)]
pub enum EditorOutcome {
    Continue,
    Save(PromptSlot),
    Cancel,
    ConfirmDiscard,
}
impl PromptEditor {
    pub fn create(order: u32) -> Self {
        Self::edit(&PromptSlot {
            id: uuid::Uuid::new_v4(),
            title: String::new(),
            body: String::new(),
            tags: Vec::new(),
            order,
        })
    }
    pub fn edit(slot: &PromptSlot) -> Self {
        Self {
            fields: [
                TextBuffer::new(slot.title.clone()),
                TextBuffer::new(slot.tags.join(", ")),
                TextBuffer::new(slot.body.clone()),
            ],
            focus: 0,
            original: slot.clone(),
        }
    }
    fn cancel_outcome(&self) -> EditorOutcome {
        if self.dirty() {
            EditorOutcome::ConfirmDiscard
        } else {
            EditorOutcome::Cancel
        }
    }
    pub fn dirty(&self) -> bool {
        self.fields[0].text != self.original.title
            || self.fields[1].text != self.original.tags.join(", ")
            || self.fields[2].text != self.original.body
    }
    pub fn handle(&mut self, event: Event) -> Result<EditorOutcome> {
        match event {
            Event::Paste(text) => self.fields[self.focus].insert(&text)?,
            Event::Key(key) if key.kind != crossterm::event::KeyEventKind::Release => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if key.code == KeyCode::Char('s') {
                        let mut next = self.original.clone();
                        next.title = self.fields[0].text.clone();
                        next.body = self.fields[2].text.clone();
                        next.tags = self.fields[1]
                            .text
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                            .collect();
                        next.normalize()?;
                        return Ok(EditorOutcome::Save(next));
                    }
                    if key.code == KeyCode::Char('c') {
                        return Ok(self.cancel_outcome());
                    }
                    return Ok(EditorOutcome::Continue);
                }
                match key.code {
                    KeyCode::Esc => {
                        return Ok(self.cancel_outcome());
                    }
                    KeyCode::Tab => self.focus = (self.focus + 1) % 3,
                    KeyCode::BackTab => self.focus = (self.focus + 2) % 3,
                    KeyCode::Enter if self.focus < 2 => self.focus += 1,
                    code => self.fields[self.focus].key(code, self.focus == 2)?,
                }
            }
            _ => (),
        }
        Ok(EditorOutcome::Continue)
    }
}
