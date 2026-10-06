use crate::{
    app::AppState,
    tmux::PanePreview,
    ui::{preview_layout, text::clip},
};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    symbols::line,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) {
    let name = app
        .sessions
        .get(app.selected)
        .map(|s| s.name.as_str())
        .unwrap_or("—");
    render_panes(frame, area, &format!(" Preview · {name} "), &app.preview);
}

/// 依 tmux 版面縮放顯示 window 內所有 pane，pane 之間以共用分隔線切開。
pub fn render_panes(frame: &mut Frame, area: Rect, title: &str, panes: &[PanePreview]) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title.to_string());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let layouts: Vec<_> = panes.iter().map(|p| p.layout.clone()).collect();
    let layout = preview_layout::compute(inner, &layouts);
    draw_separators(frame.buffer_mut(), area, &layout.separators);
    let last = layout.slots.len().saturating_sub(1);
    for (n, slot) in layout.slots.iter().enumerate() {
        let hidden = if n == last { layout.hidden } else { 0 };
        render_pane(frame, slot.area, &panes[slot.pane], hidden);
    }
}

fn render_pane(frame: &mut Frame, area: Rect, pane: &PanePreview, hidden: usize) {
    let suffix = if hidden > 0 {
        format!(" … +{hidden}")
    } else {
        String::new()
    };
    let width = area.width;
    let room = width.saturating_sub(UnicodeWidthStr::width(suffix.as_str()) as u16);
    let name = format!("{}: {}", pane.layout.index, pane.layout.command);
    let title = clip(&format!("{}{suffix}", clip(&name, room)), width);
    let style = if pane.layout.active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    // 底部空白列不算內容，避免輸出少時只看到空白。
    let mut lines: Vec<&str> = pane.text.lines().collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    // 只剩一列時最新輸出比標題重要。
    let titled = area.height >= preview_layout::MIN_HEIGHT;
    let visible = usize::from(area.height - u16::from(titled));
    let tail = &lines[lines.len().saturating_sub(visible)..];
    let text: Vec<Line> = titled
        .then(|| Line::styled(title, style))
        .into_iter()
        .chain(tail.iter().map(|l| Line::raw(clip(l, width))))
        .collect();
    frame.render_widget(Paragraph::new(text), area);
}

/// 畫分隔線，並把交叉處與碰到外框的位置接成 ┼ ├ ┤ ┬ ┴；外框上的標題文字保留。
fn draw_separators(buffer: &mut Buffer, area: Rect, separators: &[Rect]) {
    let border = |x: u16, y: u16| {
        x == area.left() || x + 1 == area.right() || y == area.top() || y + 1 == area.bottom()
    };
    let is_line = |x: i32, y: i32| {
        let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
            return false;
        };
        let position = Position { x, y };
        area.contains(position) && (border(x, y) || separators.iter().any(|r| r.contains(position)))
    };
    let glyph = |x: u16, y: u16| {
        let (x, y) = (i32::from(x), i32::from(y));
        junction(
            is_line(x, y - 1),
            is_line(x, y + 1),
            is_line(x - 1, y),
            is_line(x + 1, y),
        )
    };
    let style = Style::default().fg(Color::DarkGray);
    let mut cells = Vec::new();
    for r in separators {
        cells.extend(r.positions().map(|p| (p, true)));
        // 分隔線兩端外一格若是外框，也要接起來。
        let ends = if r.width == 1 {
            [(r.x, r.y.wrapping_sub(1)), (r.x, r.bottom())]
        } else {
            [(r.x.wrapping_sub(1), r.y), (r.right(), r.y)]
        };
        cells.extend(ends.map(|(x, y)| (Position { x, y }, false)));
    }
    for (p, own) in cells {
        if !area.contains(p) || (!own && !border(p.x, p.y)) {
            continue;
        }
        let Some(cell) = buffer.cell_mut(p) else {
            continue;
        };
        if own || [line::HORIZONTAL, line::VERTICAL].contains(&cell.symbol()) {
            let symbol = glyph(p.x, p.y);
            cell.set_symbol(symbol).set_style(style);
        }
    }
}

fn junction(up: bool, down: bool, left: bool, right: bool) -> &'static str {
    match (up, down, left, right) {
        (true, true, true, true) => line::CROSS,
        (true, true, false, true) => line::VERTICAL_RIGHT,
        (true, true, true, false) => line::VERTICAL_LEFT,
        (false, true, true, true) => line::HORIZONTAL_DOWN,
        (true, false, true, true) => line::HORIZONTAL_UP,
        (_, _, false, false) => line::VERTICAL,
        _ => line::HORIZONTAL,
    }
}
