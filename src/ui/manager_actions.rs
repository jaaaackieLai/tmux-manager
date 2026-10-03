use crate::{
    app::{AppState, DETAIL_ACTIONS, ManagerAction},
    ui::manager_mouse::ActionMenuHitMap,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, app: &AppState) -> Option<ActionMenuHitMap> {
    let session = app.sessions.get(app.selected)?;
    let width = area.width.min(56);
    let height = area.height.min(15);
    let menu = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Session 操作 ")
        .style(Style::default().fg(Color::Gray).bg(Color::Black));
    let inner = block.inner(menu);
    frame.render_widget(Clear, menu);
    frame.render_widget(block, menu);
    // 有空間時每個選項佔兩行；小視窗優先保留可捲動操作。
    let stride = if inner.height >= 9 { 2 } else { 1 };
    let footer_height = if inner.height > 4 * stride { 1 } else { 0 };
    let header_height = inner
        .height
        .saturating_sub(4 * stride + footer_height)
        .min(4);
    let action_height = (inner.height - header_height - footer_height).min(4 * stride);
    let actions = Rect::new(inner.x, inner.y + header_height, inner.width, action_height);
    let summary = app
        .ai
        .get(&session.id)
        .and_then(|result| result.as_ref().ok())
        .map(|summary| summary.summary.as_str())
        .unwrap_or("");
    frame.render_widget(
        Paragraph::new(format!(
            "{} ({})\n{} windows · created {}\nAI: {}",
            session.name,
            session.id.as_str(),
            session.windows,
            session.created,
            summary
        )),
        Rect::new(inner.x, inner.y, inner.width, header_height.min(3)),
    );
    let selected = app.detail_selected.min(DETAIL_ACTIONS.len() - 1);
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(
        List::new(DETAIL_ACTIONS.map(|(label, _)| {
            ListItem::new(if stride == 2 {
                format!("{label}\n ")
            } else {
                label.into()
            })
        }))
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(if DETAIL_ACTIONS[selected].1 == ManagerAction::Kill {
                    Color::Red
                } else {
                    Color::Yellow
                })
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        actions,
        &mut state,
    );
    let footer = Rect::new(
        inner.x,
        actions.bottom(),
        inner.width,
        inner.bottom() - actions.bottom(),
    );
    let help = if inner.width >= 50 {
        "↑↓/Tab 選擇 · Enter/左鍵 執行 · Esc/q/點外 關閉"
    } else if inner.width >= 31 {
        "↑↓ 選擇 · Enter 執行 · Esc 關閉"
    } else {
        "Esc 關閉"
    };
    frame.render_widget(Paragraph::new(help), footer);
    let rows: Vec<_> = (state.offset()..DETAIL_ACTIONS.len())
        .enumerate()
        .map(|(row, index)| {
            (
                Rect::new(actions.x, actions.y + row as u16 * stride, actions.width, 1),
                index,
            )
        })
        .take_while(|(rect, _)| rect.y < actions.bottom())
        .filter(|(rect, _)| rect.width > 0)
        .collect();
    if stride == 2 {
        for (rect, _) in &rows {
            frame.render_widget(
                Block::default().style(Style::default().bg(Color::Black)),
                Rect::new(rect.x, rect.y + 1, rect.width, 1),
            );
        }
    }
    Some(ActionMenuHitMap {
        area: menu,
        session_id: session.id.clone(),
        rows,
    })
}
