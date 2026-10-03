use crate::{Result, error::error};
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PaneId(String);
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SessionId(String);
fn valid_id(value: &str, prefix: char) -> bool {
    value.starts_with(prefix) && value.len() > 1 && value[1..].bytes().all(|b| b.is_ascii_digit())
}
impl PaneId {
    pub fn parse(value: &str) -> Result<Self> {
        if valid_id(value, '%') {
            Ok(Self(value.into()))
        } else {
            Err(error("pane ID 必須為 % 加數字，例如 %12"))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl SessionId {
    pub fn parse(value: &str) -> Result<Self> {
        if valid_id(value, '$') {
            Ok(Self(value.into()))
        } else {
            Err(error("無效的 session ID"))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub windows: u32,
    pub created: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pane {
    pub id: PaneId,
    pub description: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneCapabilities {
    pub description: String,
    pub bracketed_paste: Option<bool>,
    pub dead: bool,
    pub input_off: bool,
    pub in_mode: bool,
    pub synchronized: bool,
}
