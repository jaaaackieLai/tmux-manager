use super::model::PromptSlot;
use crate::ui::{hit_test, text};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;
use uuid::Uuid;

#[derive(Default)]
pub struct DockView {
    /// 目前頁碼；分頁每次依目前寬度與 prompts 重新計算，resize 或資料變動也不會錯位。
    page: usize,
    page_count: usize,
    hits: Vec<(Rect, Uuid)>,
    navigation: Vec<(Rect, Page)>,
}
#[derive(Clone, Copy)]
enum Page {
    Previous,
    Next,
}
/// 每個 prompt 在第二列的位置，依序填滿每頁（右側保留翻頁按鈕）。
fn paginate(slots: &[PromptSlot], area: Rect) -> Vec<Vec<(usize, Rect)>> {
    let start = area.x + area.width.min(1);
    let end = area.right().saturating_sub(8);
    let mut pages = vec![Vec::new()];
    let mut x = start;
    for (index, slot) in slots.iter().enumerate() {
        if end.saturating_sub(x) < 5 && x > start {
            pages.push(Vec::new());
            x = start;
        }
        let available = end.saturating_sub(x);
        if available < 5 {
            break;
        }
        let width = (UnicodeWidthStr::width(slot.title.as_str()) as u16)
            .saturating_add(4)
            .clamp(5, 24)
            .min(available);
        pages
            .last_mut()
            .unwrap()
            .push((index, Rect::new(x, area.y + 1, width, 1)));
        x += width + 1;
    }
    pages
}
impl DockView {
    pub fn render(&mut self, frame: &mut Frame, slots: &[PromptSlot], status: &str) {
        let area = frame.area();
        self.hits.clear();
        self.navigation.clear();
        frame.render_widget(
            Paragraph::new(" Prompts · 滑鼠點選填入上方工作區")
                .style(Style::default().fg(Color::Cyan)),
            Rect::new(area.x, area.y, area.width, area.height.min(1)),
        );
        if area.height < 2 {
            return;
        }
        if slots.is_empty() {
            frame.render_widget(
                Paragraph::new(" 尚無 prompt；回 manager 按 p 新增"),
                Rect::new(area.x, area.y + 1, area.width, 1),
            );
        }
        let pages = paginate(slots, area);
        self.page_count = pages.len();
        self.page = self.page.min(self.page_count - 1);
        for &(index, rect) in &pages[self.page] {
            let slot = &slots[index];
            let title = text::clip(&slot.title, rect.width.saturating_sub(4));
            frame.render_widget(
                Paragraph::new(format!("[ {title} ]"))
                    .style(Style::default().fg(Color::White).bg(Color::Rgb(35, 48, 65))),
                rect,
            );
            self.hits.push((rect, slot.id));
        }
        if area.width >= 12 {
            for (label, delta, page, enabled) in [
                ("[‹]", 7, Page::Previous, self.page > 0),
                ("[›]", 3, Page::Next, self.page + 1 < self.page_count),
            ] {
                let rect = Rect::new(area.right() - delta, area.y + 1, 3, 1);
                frame.render_widget(
                    Paragraph::new(label).style(Style::default().fg(if enabled {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    })),
                    rect,
                );
                if enabled {
                    self.navigation.push((rect, page));
                }
            }
        }
        if area.height > 2 {
            frame.render_widget(
                Paragraph::new(text::fold(status, area.width))
                    .style(Style::default().fg(Color::Gray)),
                Rect::new(area.x, area.y + 2, area.width, area.height - 2),
            );
        }
    }
    /// 翻頁；已在第一頁或最後一頁時不動。
    fn turn(&mut self, page: Page) {
        self.page = match page {
            Page::Next => (self.page + 1).min(self.page_count.saturating_sub(1)),
            Page::Previous => self.page.saturating_sub(1),
        };
    }
    pub fn handle_mouse(&mut self, event: MouseEvent) -> Option<Uuid> {
        match event.kind {
            MouseEventKind::ScrollDown => self.turn(Page::Next),
            MouseEventKind::ScrollUp => self.turn(Page::Previous),
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(page) = hit_test::click(&self.navigation, event) {
                    self.turn(*page);
                } else {
                    return hit_test::click(&self.hits, event).copied();
                }
            }
            _ => (),
        }
        None
    }
}
