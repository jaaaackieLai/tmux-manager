use crate::ui::text::clip;
use crate::{
    app::AppState,
    ui::{manager_layout::preferred_stride, manager_mouse::ManagerHit},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

pub struct SessionMetrics {
    pub inner: Rect,
    pub header: u16,
    pub stride: u16,
    pub capacity: usize,
}

impl SessionMetrics {
    pub fn new(area: Rect) -> Self {
        let inner = Block::default().borders(Borders::ALL).inner(area);
        let preferred = preferred_stride(inner.width);
        let header = if inner.height >= preferred + 2 { 2 } else { 0 };
        let remaining = inner.height.saturating_sub(header);
        let stride = preferred.min(remaining.max(1));
        Self {
            inner,
            header,
            stride,
            capacity: usize::from(remaining / stride),
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) -> Vec<(Rect, ManagerHit)> {
    let metrics = SessionMetrics::new(area);
    let inner = metrics.inner;
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(format!(" Sessions · {} · 單擊進入 ", app.sessions.len())),
        area,
    );
    if app.sessions.is_empty() {
        frame.render_widget(
            Paragraph::new("尚無 session · 按 n 新增").style(Style::default().fg(Color::Gray)),
            inner,
        );
        return Vec::new();
    }
    if metrics.capacity == 0 || inner.width == 0 {
        return Vec::new();
    }
    let wide = inner.width >= 70;
    let windows_width = if inner.width >= 24 { 11 } else { 0 };
    let name_width = if wide {
        app.sessions
            .iter()
            .map(|s| UnicodeWidthStr::width(s.name.as_str()))
            .max()
            .unwrap_or(16)
            .clamp(16, 28)
            .min(usize::from(inner.width.saturating_sub(18) / 2)) as u16
    } else {
        inner.width.saturating_sub(2 + windows_width)
    };
    let name_x = inner.x + inner.width.min(2);
    let windows_x = (name_x + name_width + if wide { 2 } else { 0 }).min(inner.right());
    let ai_x = (windows_x + windows_width + 3).min(inner.right());
    if metrics.header > 0 {
        frame.render_widget(
            Paragraph::new("SESSION").style(
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(name_x, inner.y, name_width, 1),
        );
        if windows_width > 0 {
            frame.render_widget(
                Paragraph::new("WINDOWS").style(Style::default().fg(Color::Gray)),
                Rect::new(
                    windows_x,
                    inner.y,
                    windows_width.min(inner.right() - windows_x),
                    1,
                ),
            );
        }
        if wide {
            frame.render_widget(
                Paragraph::new("AI 摘要").style(Style::default().fg(Color::Gray)),
                Rect::new(ai_x, inner.y, inner.right() - ai_x, 1),
            );
        }
        frame.render_widget(
            Paragraph::new("─".repeat(usize::from(inner.width)))
                .style(Style::default().fg(Color::DarkGray)),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );
    }
    let offset = app
        .selected
        .saturating_add(1)
        .saturating_sub(metrics.capacity);
    app.sessions
        .iter()
        .skip(offset)
        .take(metrics.capacity)
        .enumerate()
        .map(|(row, session)| {
            let y = inner.y + metrics.header + row as u16 * metrics.stride;
            let content_height = if wide { 1 } else { metrics.stride.min(2) };
            let rect = Rect::new(inner.x, y, inner.width, content_height);
            let selected = app.selected == offset + row;
            let background = if selected {
                Color::Rgb(35, 48, 65)
            } else {
                Color::Reset
            };
            frame.render_widget(
                Block::default().style(Style::default().bg(background)),
                rect,
            );
            if selected {
                frame.render_widget(
                    Paragraph::new("›").style(Style::default().fg(Color::Cyan).bg(background)),
                    Rect::new(inner.x, y, 1, 1),
                );
            }
            frame.render_widget(
                Paragraph::new(clip(&session.name, name_width)).style(
                    Style::default()
                        .fg(Color::White)
                        .bg(background)
                        .add_modifier(if selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Rect::new(name_x, y, name_width, 1),
            );
            if windows_width > 0 {
                frame.render_widget(
                    Paragraph::new(format!("{} windows", session.windows))
                        .style(Style::default().fg(Color::Gray).bg(background)),
                    Rect::new(
                        windows_x,
                        y,
                        windows_width.min(inner.right() - windows_x),
                        1,
                    ),
                );
            }
            let (summary, color) = match app.ai.get(&session.id) {
                Some(Ok(ai)) => (
                    ai.summary.split_whitespace().collect::<Vec<_>>().join(" "),
                    Color::Gray,
                ),
                Some(Err(_)) => ("摘要失敗".into(), Color::Red),
                None => ("—".into(), Color::DarkGray),
            };
            if wide || content_height > 1 {
                let text_area = if wide {
                    Rect::new(ai_x, y, inner.right() - ai_x, 1)
                } else {
                    Rect::new(name_x, y + 1, inner.right() - name_x, 1)
                };
                frame.render_widget(
                    Paragraph::new(clip(&summary, text_area.width))
                        .style(Style::default().fg(color).bg(background)),
                    text_area,
                );
            }
            (rect, ManagerHit::Session(session.id.clone()))
        })
        .collect()
}
