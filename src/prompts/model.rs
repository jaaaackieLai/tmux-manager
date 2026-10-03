use crate::{Result, error::error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptSlot {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub order: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub slots: Vec<PromptSlot>,
}
impl Default for PromptDocument {
    fn default() -> Self {
        Self {
            schema_version: 1,
            slots: Vec::new(),
        }
    }
}
pub const MAX_BODY_BYTES: usize = 65536;
pub fn is_forbidden_control(c: char) -> bool {
    c.is_control() && !matches!(c, '\n' | '\t')
}
pub fn normalize_body(body: &str) -> Result<String> {
    let body = body.replace("\r\n", "\n");
    if body.trim().is_empty() {
        return Err(error("prompt 內容不可只有空白"));
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(error("prompt 內容上限 64 KiB UTF-8"));
    }
    if body.chars().any(is_forbidden_control) {
        return Err(error("prompt 拒絕控制字元，只允許 LF/Tab"));
    }
    Ok(body)
}
impl PromptSlot {
    pub fn new(title: &str, body: &str, tags: Vec<String>, order: u32) -> Result<Self> {
        let mut slot = Self {
            id: Uuid::new_v4(),
            title: title.into(),
            body: body.into(),
            tags,
            order,
        };
        slot.normalize()?;
        Ok(slot)
    }
    pub fn normalize(&mut self) -> Result<()> {
        self.title = self.title.trim().to_string();
        if self.title.is_empty() || self.title.chars().any(char::is_control) {
            return Err(error("prompt 標題不可空白或包含控制字元"));
        }
        if self.tags.iter().any(|s| s.chars().any(char::is_control)) {
            return Err(error("標籤不可包含控制字元"));
        }
        self.body = normalize_body(&self.body)?;
        Ok(())
    }
}
impl PromptDocument {
    pub fn validate(&self) -> Result<()> {
        self.clone().into_normalized().map(drop)
    }
    pub fn into_normalized(mut self) -> Result<Self> {
        if self.schema_version != 1 {
            return Err(error(format!(
                "不支援 prompt schema_version={}；原檔保留",
                self.schema_version
            )));
        }
        let mut ids = std::collections::HashSet::new();
        for slot in &mut self.slots {
            if !ids.insert(slot.id) {
                return Err(error("重複的 prompt UUID"));
            }
            slot.normalize()?;
        }
        Ok(self)
    }
    pub fn search(&self, query: &str) -> Vec<&PromptSlot> {
        let query = query.to_lowercase();
        let mut slots: Vec<_> = self
            .slots
            .iter()
            .filter(|s| {
                s.title.to_lowercase().contains(&query)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&query))
            })
            .collect();
        slots.sort_by_key(|s| (s.order, s.id));
        slots
    }
}
