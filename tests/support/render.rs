use ratatui::{Terminal, backend::TestBackend};

/// 第 `y` 列的畫面文字；寬字元只取一次，不重複接上被佔用的 cell。
pub fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buffer = terminal.backend().buffer();
    let mut result = String::new();
    let mut x = 0;
    while x < buffer.area.width {
        let symbol = buffer[(x, y)].symbol();
        result.push_str(symbol);
        x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
    }
    result
}
