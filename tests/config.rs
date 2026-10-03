use std::{collections::BTreeMap, path::Path};
use tmux_manager::config::{CliOverrides, Config, migrate_legacy};

const HOME: &str = "/home/test";

#[test]
fn config_precedence_and_custom_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("custom.toml");
    std::fs::write(&path, "NEW_DEFAULT_DIR = '/toml'\nPOLL_INTERVAL = 0.2\n").unwrap();
    let env = BTreeMap::from([
        ("HOME".into(), "/home/test".into()),
        (
            "TMUX_MANAGER_CONFIG_FILE".into(),
            path.to_str().unwrap().into(),
        ),
        ("TMUX_MANAGER_NEW_DEFAULT_DIR".into(), "/env".into()),
    ]);
    let config = Config::load(&CliOverrides::default(), &env).unwrap();
    assert_eq!(config.new_default_dir, "/env");
    assert_eq!(config.prompts_path(), dir.path().join("prompts.toml"));
    let config = Config::load(
        &CliOverrides {
            new_default_dir: Some("/cli".into()),
            ..Default::default()
        },
        &env,
    )
    .unwrap();
    assert_eq!(config.new_default_dir, "/cli");
}
#[test]
fn config_preserves_empty_and_multiline_strings_and_rejects_unknown_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    Config::set(
        &path,
        Path::new(HOME),
        "NEW_DEFAULT_CMD",
        "中文\n'quoted' \"text\"",
    )
    .unwrap();
    let config = Config::read(&path).unwrap();
    assert_eq!(config.new_default_cmd, "中文\n'quoted' \"text\"");
    Config::set(&path, Path::new(HOME), "NEW_DEFAULT_CMD", "").unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(Config::set(&path, Path::new(HOME), "NOT_A_KEY", "oops").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(Config::read(&path).unwrap().new_default_cmd, "");
}
#[test]
fn invalid_bool_and_poll_interval_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    for (key, value) in [
        ("NEW_ASK_DIR", "maybe"),
        ("POLL_INTERVAL", "0"),
        ("POLL_INTERVAL", "NaN"),
        ("POLL_INTERVAL", "-1"),
    ] {
        assert!(Config::set(&path, Path::new(HOME), key, value).is_err());
        assert!(!path.exists());
    }
    Config::set(&path, Path::new(HOME), "NEW_ASK_DIR", "yes").unwrap();
    assert!(Config::read(&path).unwrap().new_ask_dir);
}
#[test]
fn legacy_migration_never_executes_and_keeps_both_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("config.sh");
    let dest = dir.path().join("config.toml");
    std::fs::write(
        &source,
        "# comment\nTMUX_MANAGER_NEW_DEFAULT_DIR=\"$HOME/project\"\nTMUX_MANAGER_NEW_ASK_DIR=1\n",
    )
    .unwrap();
    migrate_legacy(&source, &dest, Path::new(HOME)).unwrap();
    assert_eq!(
        Config::read(&dest).unwrap().new_default_dir,
        "/home/test/project"
    );
    let before = std::fs::read(&dest).unwrap();
    assert!(migrate_legacy(&source, &dest, Path::new(HOME)).is_err());
    assert_eq!(std::fs::read(&dest).unwrap(), before);
    let unsafe_dest = dir.path().join("unsafe.toml");
    for line in [
        "TMUX_MANAGER_NEW_DEFAULT_CMD=$(touch nope)",
        "TMUX_MANAGER_NEW_DEFAULT_CMD=`touch nope`",
        "source file",
        "TMUX_MANAGER_NEW_DEFAULT_DIR=$OTHER",
    ] {
        std::fs::write(&source, format!("# header\n{line}\n")).unwrap();
        let error = migrate_legacy(&source, &unsafe_dest, Path::new(HOME))
            .unwrap_err()
            .to_string();
        assert!(error.contains('2'), "{error}");
        assert!(!unsafe_dest.exists());
        assert!(std::fs::read_to_string(&source).unwrap().contains(line));
    }
}
#[test]
fn legacy_home_prefix_does_not_expand_other_variable_names() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("config.sh");
    let destination = dir.path().join("config.toml");
    std::fs::write(
        &source,
        "TMUX_MANAGER_NEW_DEFAULT_DIR=\"$HOME_PROJECT/work\"\n",
    )
    .unwrap();
    let result = migrate_legacy(&source, &destination, Path::new(HOME));
    assert!(result.is_err(), "other variable silently changed into HOME");
    assert!(!destination.exists());
}
#[test]
fn legacy_double_quoted_values_keep_shell_symbols_literally() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("config.sh");
    let destination = dir.path().join("config.toml");
    // v1 `config_set` 一律寫成雙引號；Bash 在雙引號內不解讀這些符號。
    std::fs::write(
        &source,
        "TMUX_MANAGER_NEW_DEFAULT_CMD=\"source .venv/bin/activate && claude; echo (ok) | cat > /dev/null < x\"\n",
    )
    .unwrap();
    migrate_legacy(&source, &destination, Path::new(HOME)).unwrap();
    assert_eq!(
        Config::read(&destination).unwrap().new_default_cmd,
        "source .venv/bin/activate && claude; echo (ok) | cat > /dev/null < x"
    );
    for line in [
        "TMUX_MANAGER_NEW_DEFAULT_CMD=a&&b",
        "TMUX_MANAGER_NEW_DEFAULT_CMD=\"`touch nope`\"",
        "TMUX_MANAGER_NEW_DEFAULT_CMD=\"$(touch nope)\"",
        "TMUX_MANAGER_NEW_DEFAULT_CMD=\"a\\\"b\"",
    ] {
        let unsafe_dest = dir.path().join("unsafe.toml");
        std::fs::write(&source, format!("{line}\n")).unwrap();
        assert!(
            migrate_legacy(&source, &unsafe_dest, Path::new(HOME)).is_err(),
            "{line}"
        );
        assert!(!unsafe_dest.exists());
    }
}
#[test]
fn invalid_or_empty_env_overrides_are_ignored_instead_of_aborting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "NEW_ASK_DIR = true\nPOLL_INTERVAL = 0.5\n").unwrap();
    let env = BTreeMap::from([
        (
            "TMUX_MANAGER_CONFIG_FILE".into(),
            path.to_str().unwrap().into(),
        ),
        ("TMUX_MANAGER_NEW_ASK_DIR".into(), String::new()),
        ("TMUX_MANAGER_NEW_ASK_CMD".into(), "maybe".into()),
        ("TMUX_MANAGER_POLL_INTERVAL".into(), "fast".into()),
        ("TMUX_MANAGER_NEW_DEFAULT_CMD".into(), "claude".into()),
    ]);
    let config = Config::load(&CliOverrides::default(), &env).unwrap();
    assert!(config.new_ask_dir);
    assert!(!config.new_ask_cmd);
    assert_eq!(config.poll_interval, 0.5);
    assert_eq!(config.new_default_cmd, "claude");
}
fn env_for(path: &std::path::Path) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "TMUX_MANAGER_CONFIG_FILE".into(),
        path.to_str().unwrap().into(),
    )])
}
#[test]
fn first_load_migrates_a_sibling_v1_config_sh_and_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("config.sh");
    let path = dir.path().join("config.toml");
    std::fs::write(
        &legacy,
        "TMUX_MANAGER_NEW_DEFAULT_CMD=\"claude\"\nTMUX_MANAGER_NEW_ASK_DIR=\"1\"\n",
    )
    .unwrap();
    let config = Config::load(&CliOverrides::default(), &env_for(&path)).unwrap();
    assert_eq!(config.new_default_cmd, "claude");
    assert!(config.new_ask_dir);
    assert_eq!(config.notice, None);
    assert_eq!(Config::read(&path).unwrap().new_default_cmd, "claude");
    assert!(legacy.exists());
}
#[test]
fn unmigratable_v1_config_sh_starts_with_defaults_and_a_notice() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("config.sh");
    let path = dir.path().join("config.toml");
    std::fs::write(&legacy, "source ~/.secrets\n").unwrap();
    let config = Config::load(&CliOverrides::default(), &env_for(&path)).unwrap();
    assert_eq!(config.new_default_cmd, "");
    assert!(
        config
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("config.sh"))
    );
    assert!(!path.exists());
}
#[test]
fn config_file_env_pointing_at_v1_config_sh_uses_and_migrates_the_toml_beside_it() {
    // v1 README 教使用者 export TMUX_MANAGER_CONFIG_FILE=…/config.sh；升級後不可因此無法啟動。
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("config.sh");
    std::fs::write(&legacy, "TMUX_MANAGER_NEW_DEFAULT_CMD=\"claude\"\n").unwrap();
    let config = Config::load(&CliOverrides::default(), &env_for(&legacy)).unwrap();
    assert_eq!(config.path, dir.path().join("config.toml"));
    assert_eq!(config.new_default_cmd, "claude");
    assert!(dir.path().join("config.toml").exists());
    assert!(legacy.exists());
}
#[test]
fn setting_a_key_first_migrates_the_v1_config_sh() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        dir.path().join("config.sh"),
        "TMUX_MANAGER_NEW_DEFAULT_CMD=\"claude\"\n",
    )
    .unwrap();
    Config::set(&path, Path::new(HOME), "POLL_INTERVAL", "0.5").unwrap();
    let config = Config::read(&path).unwrap();
    assert_eq!(config.new_default_cmd, "claude");
    assert_eq!(config.poll_interval, 0.5);
}
#[test]
fn setting_a_key_refuses_to_bury_an_unmigratable_v1_config_sh() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(dir.path().join("config.sh"), "source ~/.secrets\n").unwrap();
    let error = Config::set(&path, Path::new(HOME), "POLL_INTERVAL", "0.5")
        .unwrap_err()
        .to_string();
    assert!(error.contains("config.sh"), "{error}");
    assert!(
        !path.exists(),
        "預設值不可寫入，否則之後不會再嘗試遷移舊設定"
    );
}
#[test]
fn an_empty_config_file_variable_falls_back_to_the_default_path() {
    // install.sh 的 `${TMUX_MANAGER_CONFIG_FILE:-...}` 也把空值視為未設定。
    let env = BTreeMap::from([
        ("HOME".into(), HOME.into()),
        ("TMUX_MANAGER_CONFIG_FILE".into(), String::new()),
    ]);
    assert_eq!(
        Config::resolve_path(&CliOverrides::default(), &env),
        Path::new("/home/test/.config/tmux-manager/config.toml")
    );
}
#[test]
fn a_relative_config_path_is_resolved_against_the_launch_directory() {
    // popup binding 與底部列在其他 cwd 執行；相對路徑會指到別的設定檔。
    let env = BTreeMap::from([("TMUX_MANAGER_CONFIG_FILE".into(), "conf/config.toml".into())]);
    assert_eq!(
        Config::resolve_path(&CliOverrides::default(), &env),
        std::env::current_dir().unwrap().join("conf/config.toml")
    );
}
