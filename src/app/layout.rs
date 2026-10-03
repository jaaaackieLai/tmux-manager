use super::{AppState, Screen};
use crate::ui::{manager_layout::Panels, manager_mouse::ManagerHitMap};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

impl AppState {
    pub fn handle_layout_key(&mut self, key: KeyEvent, hits: &ManagerHitMap) -> bool {
        if self.screen != Screen::List || key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        let Some(panels) = hits.panels else {
            return false;
        };
        let desired = match key.code {
            KeyCode::Char('[') => Some(panels.percent().saturating_sub(5)),
            KeyCode::Char(']') => Some(panels.percent().saturating_add(5).min(100)),
            KeyCode::Char('\\') => None,
            _ => return false,
        };
        self.split_percent = desired.map(|percent| {
            let mut next = Panels::new(panels.body, self.sessions.len(), Some(percent));
            if next.sessions.height == panels.sessions.height {
                let row = if key.code == KeyCode::Char(']') {
                    panels.divider.y.saturating_add(1)
                } else {
                    panels.divider.y.saturating_sub(1)
                };
                next = Panels::new(
                    panels.body,
                    self.sessions.len(),
                    Some(panels.percent_at(row)),
                );
            }
            next.percent()
        });
        self.dragging_split = false;
        true
    }

    pub(super) fn handle_split_mouse(&mut self, event: MouseEvent, hits: &ManagerHitMap) -> bool {
        if event.kind == MouseEventKind::Down(MouseButton::Left) {
            self.dragging_split = false;
        }
        if event.kind == MouseEventKind::Up(MouseButton::Left) && self.dragging_split {
            self.dragging_split = false;
            return true;
        }
        if self.screen != Screen::List {
            return false;
        }
        let Some(panels) = hits.panels else {
            return false;
        };
        match event.kind {
            MouseEventKind::Down(MouseButton::Left)
                if panels
                    .divider
                    .contains(Position::new(event.column, event.row)) =>
            {
                self.dragging_split = true;
                true
            }
            MouseEventKind::Drag(MouseButton::Left) if self.dragging_split => {
                let next = Panels::new(
                    panels.body,
                    self.sessions.len(),
                    Some(panels.percent_at(event.row)),
                );
                self.split_percent = Some(next.percent());
                true
            }
            _ => false,
        }
    }
}
