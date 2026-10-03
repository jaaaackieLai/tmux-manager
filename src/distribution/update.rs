use super::install::{install_version, read_manifest};
use crate::{Result, error::error, storage};
use semver::Version;
use serde::Deserialize;
use std::{path::Path, time::Duration};
#[derive(Debug)]
pub struct UpdateReport {
    pub updated: bool,
    pub version: String,
}
pub fn artifact_name(os: &str, arch: &str) -> Result<String> {
    let target = match (os, arch) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => {
            return Err(error(
                "此平台沒有 release artifact；請使用 cargo install --path .",
            ));
        }
    };
    Ok(format!("tmux-manager-{target}"))
}
pub fn newer_version(current: &str, remote: &str) -> Result<bool> {
    let parse = |s: &str| {
        Version::parse(s.strip_prefix('v').unwrap_or(s))
            .map_err(|e| error(format!("無效版本號：{e}")))
    };
    Ok(parse(remote)? > parse(current)?)
}
pub fn verify_checksum(bytes: &[u8], expected: &str) -> Result<()> {
    if expected.len() != 64
        || !expected.bytes().all(|b| b.is_ascii_hexdigit())
        || storage::sha256_hex(bytes) != expected.to_lowercase()
    {
        return Err(error("checksum 不符；保留原 binary"));
    }
    Ok(())
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}
async fn download(client: &reqwest::Client, url: &str, limit: usize) -> Result<Vec<u8>> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(error("artifact 超過大小上限"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > limit {
            return Err(error("artifact 超過大小上限"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
pub async fn update(current: &Path) -> Result<UpdateReport> {
    update_from(
        current,
        "https://api.github.com/repos/jaaaackieLai/tmux-manager/releases/latest",
    )
    .await
}
pub async fn update_from(current: &Path, release_url: &str) -> Result<UpdateReport> {
    update_with_timeout(current, release_url, Duration::from_secs(15)).await
}
/// `timeout` 是連線與每次讀取的閒置上限，不限制整體下載時間，慢速網路也能完成。
pub async fn update_with_timeout(
    current: &Path,
    release_url: &str,
    timeout: Duration,
) -> Result<UpdateReport> {
    let current = std::fs::canonicalize(current)?;
    let prefix = current
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| error("找不到安裝 prefix"))?;
    let manifest = read_manifest(prefix)?;
    if current != manifest.binary {
        return Err(error(
            "此 binary 不是 installer 管理的版本；請先使用 install --prefix",
        ));
    }
    verify_checksum(&std::fs::read(&current)?, &manifest.checksum)?;
    let client = reqwest::Client::builder()
        .user_agent("tmux-manager")
        .connect_timeout(timeout)
        .read_timeout(timeout)
        .build()?;
    let release: Release = client
        .get(release_url)
        .header("accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if !newer_version(&manifest.version, &release.tag_name)? {
        return Ok(UpdateReport {
            updated: false,
            version: manifest.version,
        });
    }
    let name = artifact_name(std::env::consts::OS, std::env::consts::ARCH)?;
    let find = |name: &str| {
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .ok_or_else(|| error(format!("release 缺少 artifact {name}；保留原 binary")))
    };
    let artifact = find(&name)?;
    let checksum_asset = find(&format!("{name}.sha256"))?;
    let checksum_bytes = download(&client, &checksum_asset.browser_download_url, 4096).await?;
    let checksum_text =
        std::str::from_utf8(&checksum_bytes).map_err(|_| error("checksum 格式錯誤"))?;
    let mut fields = checksum_text.split_whitespace();
    let checksum = fields.next().ok_or_else(|| error("checksum 檔為空"))?;
    let checksum_name = fields.next().map(|s| s.trim_start_matches('*'));
    if checksum_name != Some(name.as_str()) || fields.next().is_some() {
        return Err(error("checksum artifact 名稱不符"));
    }
    let bytes = download(&client, &artifact.browser_download_url, 64 * 1024 * 1024).await?;
    verify_checksum(&bytes, checksum)?;
    // 安裝前所有下載/驗證已完成；installer 備份舊檔並原子切換。
    install_version(&bytes, prefix, &release.tag_name, Some(&manifest.checksum))?;
    Ok(UpdateReport {
        updated: true,
        version: release.tag_name,
    })
}
