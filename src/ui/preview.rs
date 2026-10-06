use crate::{app::AppState, tmux::session::PREVIEW_LINES};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) {
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!(
            " Preview · {} · 最後 {PREVIEW_LINES} 行 ",
            app.sessions
                .get(app.selected)
                .map(|s| s.name.as_str())
                .unwrap_or("—")
        ));
    let visible = usize::from(preview_block.inner(area).height);
    let lines: Vec<_> = app.preview.lines().collect();
    let preview = lines[lines.len().saturating_sub(visible)..].join("\n");
    frame.render_widget(Paragraph::new(preview).block(preview_block), area);
}
