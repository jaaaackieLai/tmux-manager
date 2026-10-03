use crate::Result;
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
/// TOML 資料檔對應的鎖檔路徑：`config.toml` → `config.toml.lock`。
/// 與 install.sh `purge_config` 的 `${config%.*}.toml.lock` 一致。
pub fn lock_path(data: &Path) -> PathBuf {
    data.with_extension("toml.lock")
}
fn open_lock_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}
pub fn lock(path: &Path) -> Result<File> {
    let file = open_lock_file(path)?;
    file.lock_exclusive()?;
    Ok(file)
}
/// 不等待：鎖已被其他程序持有時回傳 `None`。
pub fn try_lock(path: &Path) -> Result<Option<File>> {
    let file = open_lock_file(path)?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(file)),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn atomic_write(path: &Path, bytes: &[u8], overwrite: bool) -> Result<()> {
    atomic_write_mode(path, bytes, overwrite, 0o600)
}
pub fn atomic_write_mode(path: &Path, bytes: &[u8], overwrite: bool, mode: u32) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(mode))?;
    }
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    if overwrite {
        temp.persist(path).map_err(|e| e.error)?;
    } else {
        temp.persist_noclobber(path).map_err(|e| e.error)?;
    }
    File::open(parent)?.sync_all()?;
    Ok(())
}
pub fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}
pub fn remove_dir_if_empty(path: &Path) -> Result<()> {
    use std::io::ErrorKind::{DirectoryNotEmpty, NotFound};
    match fs::remove_dir(path) {
        Err(e) if !matches!(e.kind(), NotFound | DirectoryNotEmpty) => Err(e.into()),
        _ => Ok(()),
    }
}
