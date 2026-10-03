use crate::{Result, error::error, storage};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallManifest {
    pub schema_version: u32,
    pub prefix: PathBuf,
    pub binary: PathBuf,
    pub version: String,
    pub checksum: String,
    pub backup: Option<PathBuf>,
}
#[derive(Debug)]
pub struct InstallReport {
    pub binary: PathBuf,
    pub manifest: PathBuf,
}
/// Bash 版（v1.x）安裝的函式庫檔名。
const LEGACY_LIBS: [&str; 9] = [
    "actions.sh",
    "ai.sh",
    "config.sh",
    "constants.sh",
    "input.sh",
    "render.sh",
    "sessions.sh",
    "update.sh",
    "utils.sh",
];
pub(super) fn share_dir(prefix: &Path) -> PathBuf {
    prefix.join("share/tmux-manager")
}
pub fn manifest_path(prefix: &Path) -> PathBuf {
    share_dir(prefix).join("install-manifest.toml")
}
/// 只刪 Bash 版安裝過的精確路徑；資料夾清空才移除。
pub(super) fn remove_legacy(prefix: &Path) -> Result<()> {
    storage::remove_if_exists(&share_dir(prefix).join("tmux-manager"))?;
    for dir in [
        share_dir(prefix).join("lib"),
        prefix.join("bin/tmux-manager-lib"),
    ] {
        for lib in LEGACY_LIBS {
            storage::remove_if_exists(&dir.join(lib))?;
        }
        storage::remove_dir_if_empty(&dir)?;
    }
    Ok(())
}
pub fn read_manifest(prefix: &Path) -> Result<InstallManifest> {
    let manifest: InstallManifest = toml::from_str(&fs::read_to_string(manifest_path(prefix))?)?;
    manifest.validate()?;
    if fs::canonicalize(prefix)? != manifest.prefix {
        return Err(error("安裝 manifest prefix 不符"));
    }
    Ok(manifest)
}
impl InstallManifest {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || !self.prefix.is_absolute()
            || self.binary != self.prefix.join("bin/tmux-manager")
            || self
                .prefix
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(error("不合法的安裝 manifest；停止操作"));
        }
        if self.checksum.len() != 64 || !self.checksum.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(error("manifest checksum 不合法"));
        }
        if self.backup.as_ref().is_some_and(|p| {
            p.parent() != self.binary.parent()
                || !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("tmux-manager.backup-"))
        }) {
            return Err(error("manifest backup 路徑不合法"));
        }
        Ok(())
    }
}
pub fn install(source: &Path, prefix: &Path) -> Result<InstallReport> {
    install_version(&fs::read(source)?, prefix, env!("CARGO_PKG_VERSION"), None)
}
pub(crate) fn install_version(
    data: &[u8],
    prefix: &Path,
    version: &str,
    expected_checksum: Option<&str>,
) -> Result<InstallReport> {
    if data.is_empty() {
        return Err(error("安裝 binary 不可為空"));
    }
    fs::create_dir_all(prefix)?;
    let prefix = fs::canonicalize(prefix)?;
    let manifest_file = manifest_path(&prefix);
    let _lock = storage::lock(&manifest_file.with_extension("lock"))?;
    let current = if manifest_file.exists() || expected_checksum.is_some() {
        Some(read_manifest(&prefix)?)
    } else {
        None
    };
    if let (Some(expected), Some(current)) = (expected_checksum, &current) {
        if current.checksum != expected {
            return Err(error("安裝在下載期間已改變；停止更新"));
        }
        super::update::verify_checksum(&fs::read(&current.binary)?, expected)?;
    }
    let binary = prefix.join("bin/tmux-manager");
    fs::create_dir_all(binary.parent().unwrap())?;
    let backup = match fs::symlink_metadata(&binary) {
        Ok(metadata) => {
            let path =
                binary.with_file_name(format!("tmux-manager.backup-{}", uuid::Uuid::new_v4()));
            if metadata.file_type().is_symlink() {
                #[cfg(unix)]
                std::os::unix::fs::symlink(fs::read_link(&binary)?, &path)?;
                #[cfg(not(unix))]
                return Err(error("此平台不支援舊 symlink 遷移"));
            } else if metadata.is_file() {
                fs::copy(&binary, &path)?;
            } else {
                return Err(error("安裝目標不是檔案或 symlink"));
            }
            Some(path)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let manifest = InstallManifest {
        schema_version: 1,
        prefix,
        binary: binary.clone(),
        version: version.strip_prefix('v').unwrap_or(version).into(),
        checksum: storage::sha256_hex(data),
        // 不留備份；欄位保留給舊 manifest，讓之前留下的備份仍能清除。
        backup: None,
    };
    let encoded = toml::to_string_pretty(&manifest)?;
    storage::atomic_write_mode(&binary, data, true, 0o755)?;
    if let Err(e) = storage::atomic_write(&manifest_file, encoded.as_bytes(), true) {
        match &backup {
            Some(backup) => {
                fs::rename(backup, &binary)
                    .map_err(|restore| error(format!("{e}；回復失敗：{restore}")))?;
            }
            None => fs::remove_file(&binary)?,
        }
        return Err(e);
    }
    // 備份只用於上面切換失敗時回復；成功後連同舊版留下的備份一起清除。
    for path in backup.iter().chain(current.and_then(|m| m.backup).iter()) {
        storage::remove_if_exists(path)?;
    }
    remove_legacy(&manifest.prefix)?;
    Ok(InstallReport {
        binary,
        manifest: manifest_file,
    })
}
