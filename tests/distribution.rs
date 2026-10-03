use tmux_manager::distribution::{
    artifact_name, install, newer_version, read_manifest, uninstall, verify_checksum,
};
#[test]
fn install_twice_and_uninstall_preserve_user_files_and_config() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = dir.path().join("prefix");
    let source = dir.path().join("binary");
    std::fs::write(&source, b"first binary").unwrap();
    let report = install(&source, &prefix).unwrap();
    assert_eq!(std::fs::read(&report.binary).unwrap(), b"first binary");
    std::fs::write(prefix.join("bin/other"), b"user file").unwrap();
    std::fs::create_dir_all(prefix.join("config")).unwrap();
    std::fs::write(prefix.join("config/prompts.toml"), b"prompts").unwrap();
    std::fs::write(&source, b"updated binary").unwrap();
    install(&source, &prefix).unwrap();
    let manifest = read_manifest(&prefix).unwrap();
    uninstall(&manifest).unwrap();
    assert!(!report.binary.exists());
    assert!(prefix.join("bin/other").exists());
    assert!(prefix.join("config/prompts.toml").exists());
}
#[test]
#[cfg(unix)]
fn legacy_symlink_is_replaced_without_overwriting_target_or_leaving_backup() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let prefix = dir.path().join("prefix");
    let old = dir.path().join("old-script");
    let source = dir.path().join("binary");
    std::fs::write(&old, b"legacy script").unwrap();
    std::fs::write(&source, b"rust binary").unwrap();
    std::fs::create_dir_all(prefix.join("bin")).unwrap();
    symlink(&old, prefix.join("bin/tmux-manager")).unwrap();
    let report = install(&source, &prefix).unwrap();
    assert_eq!(std::fs::read(&old).unwrap(), b"legacy script");
    assert!(
        !std::fs::symlink_metadata(&report.binary)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let leftovers: Vec<_> = std::fs::read_dir(prefix.join("bin"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|name| name != "tmux-manager")
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}
#[test]
fn checksum_failure_cannot_replace_installed_binary_and_uninstall_refuses_modified_file() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("binary");
    std::fs::write(&source, b"good").unwrap();
    let report = install(&source, dir.path()).unwrap();
    assert!(verify_checksum(b"bad", &"0".repeat(64)).is_err());
    assert_eq!(std::fs::read(&report.binary).unwrap(), b"good");
    std::fs::write(&report.binary, b"not our file anymore").unwrap();
    assert!(uninstall(&read_manifest(dir.path()).unwrap()).is_err());
    assert!(report.binary.exists());
}
#[test]
fn versions_are_semantic_and_release_platforms_are_explicit() {
    assert!(newer_version("2.0.9", "v2.0.10").unwrap());
    assert!(!newer_version("2.0.0", "1.99.0").unwrap());
    assert!(newer_version("invalid", "2.0.0").is_err());
    assert_eq!(
        artifact_name("linux", "x86_64").unwrap(),
        "tmux-manager-x86_64-unknown-linux-musl"
    );
    assert_eq!(
        artifact_name("macos", "aarch64").unwrap(),
        "tmux-manager-aarch64-apple-darwin"
    );
    assert_eq!(
        artifact_name("linux", "aarch64").unwrap(),
        "tmux-manager-aarch64-unknown-linux-musl"
    );
    assert!(artifact_name("windows", "x86_64").is_err());
}
#[test]
fn tag_push_publishes_every_artifact_the_installers_download() {
    // Linux 用 musl 靜態連結：不依賴建置機的 glibc 版本，舊發行版也能升級。
    // 舊版 --update 會改跑 install.sh 下載 latest release；沒有發布就無法升級。
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = std::fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    let installer = std::fs::read_to_string(root.join("install.sh")).unwrap();
    assert!(
        workflow.contains("tags:") && workflow.contains("- 'v*'"),
        "release 要由 v* tag 觸發"
    );
    assert!(
        workflow.contains("gh release create"),
        "workflow 要建立 GitHub Release"
    );
    for (os, arch) in [
        ("linux", "x86_64"),
        ("linux", "aarch64"),
        ("macos", "x86_64"),
        ("macos", "aarch64"),
    ] {
        let target = artifact_name(os, arch)
            .unwrap()
            .trim_start_matches("tmux-manager-")
            .to_string();
        assert!(
            workflow.contains(&format!("target: {target}")),
            "release 缺 {target}"
        );
        assert!(
            installer.contains(&format!("target={target}")),
            "install.sh 缺 {target}"
        );
    }
}
fn release_server(
    checksum: &str,
    artifact: &str,
    incomplete: bool,
    trickle: Option<std::time::Duration>,
) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let name = artifact_name(std::env::consts::OS, std::env::consts::ARCH).unwrap();
    let release = serde_json::json!({"tag_name":"v999.0.0","assets":[{"name":name,"browser_download_url":format!("{base}/binary")},{"name":format!("{name}.sha256"),"browser_download_url":format!("{base}/checksum")}]});
    let responses = [
        release.to_string(),
        format!("{checksum}  {name}\n"),
        artifact.into(),
    ];
    let thread = std::thread::spawn(move || {
        for (index, body) in responses.iter().enumerate() {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 8192];
            let _ = stream.read(&mut buf);
            let length = if incomplete && index == 2 {
                body.len() + 100
            } else {
                body.len()
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{body}"
            );
            match trickle.filter(|_| index == 2) {
                Some(delay) => {
                    let (head, body) = response.split_at(response.len() - body.len());
                    stream.write_all(head.as_bytes()).unwrap();
                    for byte in body.bytes() {
                        stream.flush().unwrap();
                        std::thread::sleep(delay);
                        stream.write_all(&[byte]).unwrap();
                    }
                }
                None => stream.write_all(response.as_bytes()).unwrap(),
            }
        }
    });
    (base, thread)
}
#[tokio::test]
async fn update_installs_verified_artifact_and_records_remote_version() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::write(&source, b"good").unwrap();
    let report = install(&source, dir.path()).unwrap();
    let (url, thread) = release_server(
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "abc",
        false,
        None,
    );
    let result = tmux_manager::distribution::update_from(&report.binary, &url)
        .await
        .unwrap();
    thread.join().unwrap();
    assert!(result.updated);
    assert_eq!(std::fs::read(&report.binary).unwrap(), b"abc");
    assert_eq!(read_manifest(dir.path()).unwrap().version, "999.0.0");
}
#[tokio::test]
async fn interrupted_or_bad_checksum_download_keeps_existing_install() {
    for incomplete in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::write(&source, b"good").unwrap();
        let report = install(&source, dir.path()).unwrap();
        let before = std::fs::read(&report.manifest).unwrap();
        let (url, thread) = release_server(&"0".repeat(64), "bad", incomplete, None);
        assert!(
            tmux_manager::distribution::update_from(&report.binary, &url)
                .await
                .is_err()
        );
        thread.join().unwrap();
        assert_eq!(std::fs::read(&report.binary).unwrap(), b"good");
        assert_eq!(std::fs::read(&report.manifest).unwrap(), before);
    }
}
#[test]
fn bash_constants_and_readme_badge_follow_cargo_version() {
    let version = env!("CARGO_PKG_VERSION");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let constants = std::fs::read_to_string(root.join("lib/constants.sh")).unwrap();
    assert!(
        constants.contains(&format!("readonly VERSION=\"{version}\"")),
        "舊版 --update 比對 lib/constants.sh，版本不一致就不會升級到 Rust 版"
    );
    let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
    assert!(
        readme
            .lines()
            .nth(2)
            .unwrap()
            .contains(&format!("version-{version}-"))
    );
}
#[tokio::test]
async fn slow_but_steady_artifact_download_is_not_cut_off_by_a_total_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::write(&source, b"good").unwrap();
    let report = install(&source, dir.path()).unwrap();
    let (url, thread) = release_server(
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "abc",
        false,
        Some(std::time::Duration::from_millis(700)),
    );
    tmux_manager::distribution::update_with_timeout(
        &report.binary,
        &url,
        std::time::Duration::from_secs(1),
    )
    .await
    .unwrap();
    thread.join().unwrap();
    assert_eq!(std::fs::read(&report.binary).unwrap(), b"abc");
}
