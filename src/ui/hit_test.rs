use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
/// 左鍵按下時，回傳第一個包含點擊位置的目標。
pub fn click<T>(rows: &[(Rect, T)], event: MouseEvent) -> Option<&T> {
    if event.kind != MouseEventKind::Down(MouseButton::Left) {
        return None;
    }
    rows.iter()
        .find(|(rect, _)| rect.contains(Position::new(event.column, event.row)))
        .map(|(_, target)| target)
}
/// 捲動清單中可見項目各佔一列，對應到 `area` 內的點擊範圍。
pub fn list_rows<T>(
    area: Rect,
    offset: usize,
    items: impl IntoIterator<Item = T>,
) -> impl Iterator<Item = (Rect, T)> {
    items
        .into_iter()
        .skip(offset)
        .take(usize::from(area.height))
        .enumerate()
        .map(move |(row, item)| (Rect::new(area.x, area.y + row as u16, area.width, 1), item))
}
