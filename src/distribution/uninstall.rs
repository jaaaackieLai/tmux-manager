use super::install::{InstallManifest, manifest_path, read_manifest, remove_legacy, share_dir};
use crate::{Result, error::error, storage};
use std::path::Path;
pub fn uninstall(manifest: &InstallManifest) -> Result<()> {
    manifest.validate()?;
    let path = manifest_path(&manifest.prefix);
    let _lock = storage::lock(&path.with_extension("lock"))?;
    let current = read_manifest(&manifest.prefix)?;
    if current.checksum != manifest.checksum {
        return Err(error("安裝狀態已變更，請重新載入 manifest"));
    }
    let metadata = std::fs::symlink_metadata(&current.binary)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(error("binary 已被替換為其他檔案；不移除"));
    }
    let bytes = std::fs::read(&current.binary)?;
    if storage::sha256_hex(&bytes) != current.checksum {
        return Err(error("binary 已被修改；不移除非本工具檔案"));
    }
    std::fs::remove_file(&current.binary)?;
    std::fs::remove_file(&path)?;
    if let Some(backup) = &current.backup {
        storage::remove_if_exists(backup)?;
    }
    remove_legacy(&current.prefix)?;
    storage::remove_if_exists(&path.with_extension("lock"))?;
    storage::remove_dir_if_empty(&share_dir(&current.prefix))
}
/// `--purge`：刪除設定、prompts 及其 lock；只刪精確檔名，資料夾清空才移除。
pub fn purge_user_data(config: &Path) -> Result<()> {
    let dir = config.parent().unwrap_or_else(|| Path::new("."));
    let prompts = crate::config::prompts_path_for(config);
    for path in [
        config.to_path_buf(),
        storage::lock_path(config),
        storage::lock_path(&prompts),
        prompts,
    ] {
        storage::remove_if_exists(&path)?;
    }
    // Bash 版設定只在預設的 tmux-manager 資料夾內才視為本工具檔案。
    if dir.file_name().is_some_and(|n| n == "tmux-manager") {
        storage::remove_if_exists(&dir.join("config.sh"))?;
    }
    storage::remove_dir_if_empty(dir)
}
