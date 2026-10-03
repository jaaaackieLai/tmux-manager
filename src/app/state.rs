use crate::ui::manager_mouse::{ManagerHit, ManagerHitMap};
use crate::{
    ai::AiSummary,
    tmux::{Session, SessionId},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
use std::collections::HashMap;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    List,
    Detail,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManagerAction {
    None,
    Quit,
    New,
    Refresh,
    Attach,
    Rename,
    Kill,
    Prompts,
}
/// 列表上的操作選單；`None` 表示關閉選單。
pub const DETAIL_ACTIONS: [(&str, ManagerAction); 4] = [
    ("attach", ManagerAction::Attach),
    ("rename", ManagerAction::Rename),
    ("kill", ManagerAction::Kill),
    ("back", ManagerAction::None),
];
#[derive(Default)]
pub struct AppState {
    pub sessions: Vec<Session>,
    pub selected: usize,
    pub detail_selected: usize,
    pub screen: Screen,
    pub ai: HashMap<SessionId, std::result::Result<AiSummary, String>>,
    pub generation: u64,
    pub preview: String,
    pub status: String,
    pub split_percent: Option<u16>,
    pub dragging_split: bool,
    /// 背景刷新錯誤，與操作訊息 `status` 分開保存，刷新恢復時清除。
    pub refresh_error: Option<String>,
}
impl AppState {
    pub fn handle_mouse(&mut self, event: MouseEvent, hits: &ManagerHitMap) -> ManagerAction {
        // 兩個畫面的捲輪都等同上下鍵；分隔線拖曳不處理捲輪。
        let scroll = match event.kind {
            MouseEventKind::ScrollUp => Some(KeyCode::Up),
            MouseEventKind::ScrollDown => Some(KeyCode::Down),
            _ => None,
        };
        if let Some(code) = scroll {
            return self.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
        }
        if self.screen == Screen::Detail {
            let Some(menu) = &hits.action_menu else {
                return ManagerAction::None;
            };
            if event.kind != MouseEventKind::Down(MouseButton::Left) {
                return ManagerAction::None;
            }
            if !menu.area.contains(Position::new(event.column, event.row)) {
                self.screen = Screen::List;
                self.dragging_split = false;
            } else if self.selected_id() == Some(&menu.session_id) {
                if let Some(index) = crate::ui::hit_test::click(&menu.rows, event) {
                    self.detail_selected = *index;
                    return self.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                }
            }
            return ManagerAction::None;
        }
        if self.handle_split_mouse(event, hits) {
            return ManagerAction::None;
        }
        match hits.click(event) {
            Some(ManagerHit::Prompts) => return ManagerAction::Prompts,
            Some(ManagerHit::Session(id)) => {
                if let Some(index) = self.sessions.iter().position(|session| &session.id == id) {
                    self.selected = index;
                    return self.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                }
            }
            _ => (),
        }
        ManagerAction::None
    }
    /// 狀態列文字：操作訊息與背景刷新錯誤並列，彼此不覆蓋。
    pub fn status_line(&self) -> String {
        match &self.refresh_error {
            Some(error) if self.status.is_empty() => error.clone(),
            Some(error) => format!("{} · {error}", self.status),
            None => self.status.clone(),
        }
    }
    /// 記錄背景刷新錯誤；回傳畫面是否需要重畫。
    pub fn show_refresh_error(&mut self, message: String) -> bool {
        let changed = self.refresh_error.as_ref() != Some(&message);
        self.refresh_error = Some(message);
        changed
    }
    /// 刷新恢復時清除刷新錯誤；回傳畫面是否需要重畫。
    pub fn refresh_succeeded(&mut self) -> bool {
        self.refresh_error.take().is_some()
    }
    pub fn selected_id(&self) -> Option<&SessionId> {
        self.sessions.get(self.selected).map(|s| &s.id)
    }
    pub fn replace_sessions(&mut self, sessions: Vec<Session>) -> bool {
        let changed = self.sessions != sessions;
        let kept = self
            .selected_id()
            .and_then(|id| sessions.iter().position(|s| &s.id == id));
        self.selected = kept.unwrap_or(self.selected.min(sessions.len().saturating_sub(1)));
        // 詳細頁的 session 消失時回到列表，避免操作改指向其他 session。
        if kept.is_none() {
            self.screen = Screen::List;
        }
        self.ai.retain(|id, _| sessions.iter().any(|s| &s.id == id));
        self.sessions = sessions;
        if self.sessions.is_empty() {
            self.preview.clear();
        }
        changed
    }
    pub fn apply_ai(
        &mut self,
        id: SessionId,
        generation: u64,
        result: std::result::Result<AiSummary, String>,
    ) -> bool {
        if generation != self.generation || !self.sessions.iter().any(|s| s.id == id) {
            return false;
        }
        self.ai.insert(id, result);
        true
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> ManagerAction {
        self.dragging_split = false;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return ManagerAction::Quit;
        }
        if self.screen == Screen::List {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return ManagerAction::Quit,
                KeyCode::Char('n') => return ManagerAction::New,
                KeyCode::Char('f') => return ManagerAction::Refresh,
                KeyCode::Char('p') => return ManagerAction::Prompts,
                KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                KeyCode::Down => {
                    self.selected = (self.selected + 1).min(self.sessions.len().saturating_sub(1))
                }
                KeyCode::Tab if !self.sessions.is_empty() => {
                    self.selected = (self.selected + 1) % self.sessions.len()
                }
                KeyCode::Enter if !self.sessions.is_empty() => {
                    self.screen = Screen::Detail;
                    self.detail_selected = 0;
                }
                _ => (),
            }
        } else {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::List,
                KeyCode::Char('a') => return ManagerAction::Attach,
                KeyCode::Char('r') => return ManagerAction::Rename,
                KeyCode::Char('k') => return ManagerAction::Kill,
                KeyCode::Char('p') => return ManagerAction::Prompts,
                KeyCode::Up => {
                    self.detail_selected =
                        (self.detail_selected + DETAIL_ACTIONS.len() - 1) % DETAIL_ACTIONS.len()
                }
                KeyCode::Down | KeyCode::Tab => {
                    self.detail_selected = (self.detail_selected + 1) % DETAIL_ACTIONS.len()
                }
                KeyCode::Enter => match &DETAIL_ACTIONS[self.detail_selected].1 {
                    ManagerAction::None => self.screen = Screen::List,
                    action => return action.clone(),
                },
                _ => (),
            }
        }
        ManagerAction::None
    }
}
