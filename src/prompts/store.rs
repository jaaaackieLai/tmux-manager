use super::PromptDocument;
use crate::{Error, Result, storage};
use std::{
    fs,
    path::{Path, PathBuf},
};
const MISSING: &str = "missing";
#[derive(Clone, Debug)]
pub struct PromptSnapshot {
    pub document: PromptDocument,
    pub revision: String,
}
#[derive(Clone)]
pub struct PromptStore {
    path: PathBuf,
}
impl PromptStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn load(&self) -> Result<PromptSnapshot> {
        Ok(self
            .read_unless(None)?
            .expect("沒有已知 revision 時一定會載入"))
    }
    /// 檔案內容與 `revision` 相同時回傳 `None`，省下解析。
    pub fn reload_if_changed(&self, revision: &str) -> Result<Option<PromptSnapshot>> {
        self.read_unless(Some(revision))
    }
    fn read_unless(&self, known: Option<&str>) -> Result<Option<PromptSnapshot>> {
        let contents = match fs::read(&self.path) {
            Ok(contents) => contents,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok((known != Some(MISSING)).then(|| PromptSnapshot {
                    document: PromptDocument::default(),
                    revision: MISSING.into(),
                }));
            }
            Err(e) => return Err(e.into()),
        };
        let revision = storage::sha256_hex(&contents);
        if known == Some(revision.as_str()) {
            return Ok(None);
        }
        let text = std::str::from_utf8(&contents)
            .map_err(|_| crate::error::error("prompt 檔案不是 UTF-8；原檔保留"))?;
        let document = toml::from_str::<PromptDocument>(text)?.into_normalized()?;
        Ok(Some(PromptSnapshot { document, revision }))
    }
    pub fn commit(&self, expected_revision: &str, next: &PromptDocument) -> Result<PromptSnapshot> {
        let normalized = next.clone().into_normalized()?;
        let _lock = storage::lock(&storage::lock_path(&self.path))?;
        let current = self.load()?;
        if current.revision != expected_revision {
            return Err(Error::Conflict);
        }
        let encoded = toml::to_string_pretty(&normalized)?;
        storage::atomic_write(&self.path, encoded.as_bytes(), true)?;
        Ok(PromptSnapshot {
            revision: storage::sha256_hex(encoded.as_bytes()),
            document: normalized,
        })
    }
}
impl PromptStore {
    /// 背景重讀：有新內容就換掉 snapshot；讀取失敗時把錯誤記在 `error`（由畫面顯示），
    /// 恢復後清除。回傳畫面是否需要重畫。
    pub fn reload_into(&self, snapshot: &mut PromptSnapshot, error: &mut Option<String>) -> bool {
        match self.reload_if_changed(&snapshot.revision) {
            Ok(next) => {
                let recovered = error.take().is_some();
                let replaced = next.map(|next| *snapshot = next).is_some();
                recovered || replaced
            }
            Err(e) => {
                let message = Some(e.to_string());
                let changed = *error != message;
                *error = message;
                changed
            }
        }
    }
    /// 取得要貼上的 slot：先重讀，絕不貼上已刪除或被外部修改的舊內容。
    pub fn current_slot(
        &self,
        snapshot: &mut PromptSnapshot,
        id: uuid::Uuid,
    ) -> Result<super::PromptSlot> {
        if let Some(next) = self.reload_if_changed(&snapshot.revision)? {
            *snapshot = next;
        }
        snapshot
            .document
            .slots
            .iter()
            .find(|s| s.id == id)
            .cloned()
            .ok_or_else(|| crate::error::error("此 prompt 已被移除；列表已更新"))
    }
}
