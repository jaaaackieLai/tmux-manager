use crate::{
    app::{AppState, Screen},
    ui::{
        manager_layout::Panels,
        manager_mouse::{ManagerHit, ManagerHitMap},
    },
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Paragraph, Wrap},
};
pub fn render(frame: &mut Frame, app: &AppState) -> ManagerHitMap {
    let mut hits = ManagerHitMap::default();
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().fg(Color::Gray)),
        area,
    );
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(format!(" tmux-manager v{}", env!("CARGO_PKG_VERSION")))
            .style(Style::default().fg(Color::Cyan)),
        parts[0],
    );
    if parts[0].height >= 2 {
        let label = "[p] Prompt Slots";
        let button = Rect::new(
            parts[0].x,
            parts[0].y + 1,
            parts[0].width.min(label.len() as u16),
            1,
        );
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(Color::Yellow)),
            button,
        );
        hits.rows.push((button, ManagerHit::Prompts));
    }
    {
        let panels = Panels::new(parts[1], app.sessions.len(), app.split_percent);
        hits.panels = Some(panels);
        let body = [panels.sessions, panels.preview];
        frame.render_widget(
            Paragraph::new(format!(
                "── ↕ 拖曳調整 · [ / ] 縮放 · \\ 自動 · {} ──",
                if app.split_percent.is_some() {
                    "手動"
                } else {
                    "自動"
                }
            ))
            .style(Style::default().fg(if app.dragging_split {
                Color::Cyan
            } else {
                Color::DarkGray
            })),
            panels.divider,
        );
        hits.rows
            .extend(crate::ui::manager_sessions::render(frame, body[0], app));
        crate::ui::preview::render(frame, body[1], app);
    }
    let keys = if app.screen == Screen::List {
        "[↑↓/Tab] 選擇 [Enter] 操作 [p] prompts [n] 新增 [f] 更新 [q] 離開"
    } else {
        "[a] attach [r] 改名 [k] 刪除 [p] prompts [Esc] 關閉"
    };
    frame.render_widget(
        Paragraph::new(format!("{keys}\n{}", app.status_line())).wrap(Wrap { trim: false }),
        parts[2],
    );
    if app.screen == Screen::Detail {
        hits.action_menu = crate::ui::manager_actions::render(frame, parts[1], app);
    }
    hits
}
