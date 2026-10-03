use crate::{
    Result,
    prompts::editor::TextBuffer,
    ui::terminal::{Input, UiTerminal},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
pub async fn text(
    terminal: &mut UiTerminal,
    input: &mut Input,
    title: &str,
    initial: &str,
) -> Result<Option<String>> {
    let mut value = TextBuffer::new(initial.into());
    let mut dirty = true;
    loop {
        if dirty {
            terminal.draw(|frame| {
                let area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(4), Constraint::Min(1)])
                    .split(frame.area());
                frame.render_widget(
                    Paragraph::new(value.text.as_str())
                        .block(Block::default().borders(Borders::ALL).title(title))
                        .style(Style::default().fg(Color::Yellow)),
                    area[0],
                );
                frame.render_widget(
                    Paragraph::new("[Enter] 確認 [Esc] 取消 [Ctrl-U] 清空"),
                    area[1],
                );
            })?;
            dirty = false;
        }
        match input.next_or_abort().await? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('c') => return Err(crate::error::error("操作已中止")),
                        KeyCode::Char('u') => value = TextBuffer::new(String::new()),
                        _ => (),
                    }
                } else {
                    match key.code {
                        KeyCode::Esc => return Ok(None),
                        KeyCode::Enter => return Ok(Some(value.text)),
                        code => value.key(code, false)?,
                    }
                }
                dirty = true;
            }
            Event::Paste(paste) => {
                if paste.contains(['\n', '\r']) {
                    continue;
                }
                value.insert(&paste)?;
                dirty = true;
            }
            Event::Resize(_, _) => dirty = true,
            _ => (),
        }
    }
}
pub async fn confirm(terminal: &mut UiTerminal, input: &mut Input, question: &str) -> Result<bool> {
    terminal.draw(|f| {
        f.render_widget(
            Paragraph::new(format!("{question} [y/N]")).style(Style::default().fg(Color::Red)),
            f.area(),
        )
    })?;
    loop {
        if let Event::Key(key) = input.next_or_abort().await? {
            if key.kind == KeyEventKind::Press {
                return Ok(matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')));
            }
        }
    }
}
