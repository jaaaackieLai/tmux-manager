use std::path::Path;
use tmux_manager::distribution::{install, purge_user_data, read_manifest, uninstall};
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
fn legacy_install(prefix: &Path) {
    let share = prefix.join("share/tmux-manager");
    std::fs::create_dir_all(share.join("lib")).unwrap();
    std::fs::create_dir_all(prefix.join("bin")).unwrap();
    std::fs::write(share.join("tmux-manager"), b"#!/usr/bin/env bash").unwrap();
    for lib in LEGACY_LIBS {
        std::fs::write(share.join("lib").join(lib), b"# bash").unwrap();
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(share.join("tmux-manager"), prefix.join("bin/tmux-manager"))
        .unwrap();
}
fn tmux_manager_entries(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("tmux-manager"))
        .collect()
}
#[test]
#[cfg(unix)]
fn upgrading_bash_install_removes_legacy_files_without_backup() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = dir.path().join("prefix");
    legacy_install(&prefix);
    std::fs::write(
        prefix.join("share/tmux-manager/lib/user-notes.txt"),
        b"keep",
    )
    .unwrap();
    let source = dir.path().join("binary");
    std::fs::write(&source, b"rust binary").unwrap();
    install(&source, &prefix).unwrap();
    assert_eq!(tmux_manager_entries(&prefix.join("bin")), ["tmux-manager"]);
    let share = prefix.join("share/tmux-manager");
    assert!(!share.join("tmux-manager").exists());
    for lib in LEGACY_LIBS {
        assert!(!share.join("lib").join(lib).exists(), "{lib} 殘留");
    }
    assert!(
        share.join("lib/user-notes.txt").exists(),
        "非本工具檔案不可刪"
    );
}
#[test]
fn repeated_installs_leave_no_backup() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = dir.path().join("prefix");
    let source = dir.path().join("binary");
    for content in ["v1", "v2", "v3"] {
        std::fs::write(&source, content).unwrap();
        install(&source, &prefix).unwrap();
    }
    assert_eq!(tmux_manager_entries(&prefix.join("bin")), ["tmux-manager"]);
    assert!(read_manifest(&prefix).unwrap().backup.is_none());
}
#[test]
fn install_removes_backup_left_by_older_version() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = dir.path().join("prefix");
    let source = dir.path().join("binary");
    std::fs::write(&source, "v1").unwrap();
    install(&source, &prefix).unwrap();
    let manifest = read_manifest(&prefix).unwrap();
    let old_backup = manifest.binary.with_file_name("tmux-manager.backup-old");
    std::fs::write(&old_backup, "v0").unwrap();
    let manifest_file = prefix.join("share/tmux-manager/install-manifest.toml");
    let mut text = std::fs::read_to_string(&manifest_file).unwrap();
    text.push_str(&format!(
        "backup = {:?}\n",
        old_backup.display().to_string()
    ));
    std::fs::write(&manifest_file, text).unwrap();
    assert_eq!(
        read_manifest(&prefix).unwrap().backup,
        Some(old_backup.clone())
    );
    std::fs::write(&source, "v2").unwrap();
    install(&source, &prefix).unwrap();
    assert!(!old_backup.exists());
    assert_eq!(tmux_manager_entries(&prefix.join("bin")), ["tmux-manager"]);
}
#[test]
#[cfg(unix)]
fn uninstall_after_bash_upgrade_leaves_no_residue() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = dir.path().join("prefix");
    legacy_install(&prefix);
    std::fs::write(prefix.join("bin/other-tool"), b"user").unwrap();
    let source = dir.path().join("binary");
    for content in ["v1", "v2"] {
        std::fs::write(&source, content).unwrap();
        install(&source, &prefix).unwrap();
    }
    uninstall(&read_manifest(&prefix).unwrap()).unwrap();
    assert!(tmux_manager_entries(&prefix.join("bin")).is_empty());
    assert!(prefix.join("bin/other-tool").exists());
    assert!(!prefix.join("share/tmux-manager").exists());
}
#[test]
fn purge_removes_only_known_user_data_files() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join("tmux-manager");
    std::fs::create_dir_all(&config_dir).unwrap();
    for name in [
        "config.toml",
        "config.toml.lock",
        "prompts.toml",
        "prompts.toml.lock",
        "config.sh",
    ] {
        std::fs::write(config_dir.join(name), b"x").unwrap();
    }
    purge_user_data(&config_dir.join("config.toml")).unwrap();
    assert!(!config_dir.exists());
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), b"x").unwrap();
    std::fs::write(config_dir.join("notes.md"), b"keep").unwrap();
    purge_user_data(&config_dir.join("config.toml")).unwrap();
    assert!(!config_dir.join("config.toml").exists());
    assert!(config_dir.join("notes.md").exists());
}
