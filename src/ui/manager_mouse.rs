use crate::tmux::SessionId;
use crossterm::event::MouseEvent;
use ratatui::layout::Rect;

#[derive(Clone, Debug)]
pub enum ManagerHit {
    Prompts,
    Session(SessionId),
}

#[derive(Default)]
pub struct ManagerHitMap {
    pub rows: Vec<(Rect, ManagerHit)>,
    pub panels: Option<super::manager_layout::Panels>,
    pub action_menu: Option<ActionMenuHitMap>,
}

pub struct ActionMenuHitMap {
    pub area: Rect,
    pub session_id: SessionId,
    pub rows: Vec<(Rect, usize)>,
}

impl ManagerHitMap {
    pub fn click(&self, event: MouseEvent) -> Option<&ManagerHit> {
        super::hit_test::click(&self.rows, event)
    }
}
