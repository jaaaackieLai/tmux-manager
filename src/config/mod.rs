mod legacy;
use crate::{Result, error::error, storage};
pub use legacy::{MigrationReport, migrate_legacy};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
pub type EnvMap = BTreeMap<String, String>;
pub fn home_dir(env: &EnvMap) -> PathBuf {
    PathBuf::from(env.get("HOME").map(String::as_str).unwrap_or("."))
}
#[derive(Clone, Debug, Default, clap::Args)]
pub struct CliOverrides {
    #[arg(long, global = true)]
    pub config_file: Option<PathBuf>,
    #[arg(long, global = true)]
    pub new_default_dir: Option<String>,
    #[arg(long, global = true)]
    pub new_default_cmd: Option<String>,
    #[arg(long, global = true)]
    pub new_ask_dir: Option<bool>,
    #[arg(long, global = true)]
    pub new_ask_cmd: Option<bool>,
    #[arg(long, global = true)]
    pub poll_interval: Option<f64>,
    #[arg(long, global = true)]
    pub ai_model: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Config {
    pub new_default_dir: String,
    pub new_default_cmd: String,
    pub new_ask_dir: bool,
    pub new_ask_cmd: bool,
    pub poll_interval: f64,
    pub ai_model: String,
    #[serde(skip)]
    pub path: PathBuf,
    /// 啟動時要顯示給使用者的設定提示（例如舊版設定遷移失敗）。
    #[serde(skip)]
    pub notice: Option<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            new_default_dir: String::new(),
            new_default_cmd: String::new(),
            new_ask_dir: false,
            new_ask_cmd: false,
            poll_interval: 0.2,
            ai_model: "claude-haiku-4-5-20251001".into(),
            path: PathBuf::new(),
            notice: None,
        }
    }
}
impl Config {
    /// 回傳絕對路徑；指向 Bash 版 `*.sh` 時改用同名 `*.toml`，第一次讀取會自動遷移。
    pub fn resolve_path(cli: &CliOverrides, env: &EnvMap) -> PathBuf {
        let path = cli
            .config_file
            .clone()
            .or_else(|| {
                env.get("TMUX_MANAGER_CONFIG_FILE")
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| {
                let root = env
                    .get("XDG_CONFIG_HOME")
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home_dir(env).join(".config"));
                root.join("tmux-manager/config.toml")
            });
        let path = if path.extension().is_some_and(|e| e == "sh") {
            path.with_extension("toml")
        } else {
            path
        };
        // popup binding 與底部列在其他 cwd 執行，一律傳絕對路徑。
        std::path::absolute(&path).unwrap_or(path)
    }
    pub fn read(path: &Path) -> Result<Self> {
        let mut config = match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str::<Self>(&contents)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => return Err(e.into()),
        };
        config.path = path.into();
        config.validate()?;
        Ok(config)
    }
    pub fn load(cli: &CliOverrides, env: &EnvMap) -> Result<Self> {
        let path = Self::resolve_path(cli, env);
        // 遷移失敗不阻止啟動：以預設值執行並在狀態列說明。
        let notice = migrate_sibling_legacy(&path, &home_dir(env)).err();
        let mut config = Self::read(&path)?;
        config.notice = notice.map(|e| format!("{e}；暫用預設值"));
        // 與 Bash 版相同：環境變數格式錯誤時忽略，不阻止啟動。
        for key in Self::keys() {
            if let Some(value) = env.get(&format!("TMUX_MANAGER_{key}")) {
                let mut next = config.clone();
                if next.assign(key, value).is_ok() {
                    config = next;
                }
            }
        }
        if let Some(value) = &cli.new_default_dir {
            config.new_default_dir = value.clone();
        }
        if let Some(value) = &cli.new_default_cmd {
            config.new_default_cmd = value.clone();
        }
        if let Some(value) = cli.new_ask_dir {
            config.new_ask_dir = value;
        }
        if let Some(value) = cli.new_ask_cmd {
            config.new_ask_cmd = value;
        }
        if let Some(value) = cli.poll_interval {
            config.poll_interval = value;
        }
        if let Some(value) = &cli.ai_model {
            config.ai_model = value.clone();
        }
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<()> {
        if !self.poll_interval.is_finite() || self.poll_interval <= 0.0 {
            return Err(error("POLL_INTERVAL 必須為有限正數"));
        }
        if self.ai_model.trim().is_empty() {
            return Err(error("AI_MODEL 不可空白"));
        }
        Ok(())
    }
    pub fn keys() -> &'static [&'static str] {
        &[
            "NEW_DEFAULT_DIR",
            "NEW_DEFAULT_CMD",
            "NEW_ASK_DIR",
            "NEW_ASK_CMD",
            "POLL_INTERVAL",
            "AI_MODEL",
        ]
    }
    pub fn get(&self, key: &str) -> Result<String> {
        match key {
            "NEW_DEFAULT_DIR" => Ok(self.new_default_dir.clone()),
            "NEW_DEFAULT_CMD" => Ok(self.new_default_cmd.clone()),
            "NEW_ASK_DIR" => Ok(self.new_ask_dir.to_string()),
            "NEW_ASK_CMD" => Ok(self.new_ask_cmd.to_string()),
            "POLL_INTERVAL" => Ok(self.poll_interval.to_string()),
            "AI_MODEL" => Ok(self.ai_model.clone()),
            _ => Err(error(format!("未知設定鍵：{key}"))),
        }
    }
    pub fn assign(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "NEW_DEFAULT_DIR" => self.new_default_dir = value.into(),
            "NEW_DEFAULT_CMD" => self.new_default_cmd = value.into(),
            "NEW_ASK_DIR" => self.new_ask_dir = parse_bool(value)?,
            "NEW_ASK_CMD" => self.new_ask_cmd = parse_bool(value)?,
            "POLL_INTERVAL" => {
                self.poll_interval = value
                    .parse()
                    .map_err(|_| error("POLL_INTERVAL 必須為正數"))?
            }
            "AI_MODEL" => self.ai_model = value.into(),
            _ => return Err(error(format!("未知設定鍵：{key}"))),
        }
        self.validate()
    }
    pub fn set(path: &Path, home: &Path, key: &str, value: &str) -> Result<()> {
        let _lock = storage::lock(&storage::lock_path(path))?;
        // 遷移失敗時不寫入：否則預設值會蓋住舊設定，之後也不會再嘗試遷移。
        migrate_sibling_legacy(path, home)?;
        let mut config = Self::read(path)?;
        config.assign(key, value)?;
        storage::atomic_write(path, toml::to_string_pretty(&config)?.as_bytes(), true)
    }
    pub fn prompts_path(&self) -> PathBuf {
        prompts_path_for(&self.path)
    }
}
/// 升級後第一次使用：設定檔不存在但有同名的 Bash 版 `*.sh` 時轉成 TOML（原檔保留）。
pub fn migrate_sibling_legacy(path: &Path, home: &Path) -> Result<()> {
    let legacy = path.with_extension("sh");
    if path.exists() || !legacy.is_file() {
        return Ok(());
    }
    migrate_legacy(&legacy, path, home).map(drop).map_err(|e| {
        error(format!(
            "{} 無法自動轉換（{e}）；修正後可執行 migrate-config",
            legacy.display()
        ))
    })
}
/// prompts 檔與設定檔放在同一個資料夾。
pub fn prompts_path_for(config: &Path) -> PathBuf {
    config
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("prompts.toml")
}
pub fn parse_bool(value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(error("布林值須為 true/false、1/0、yes/no 或 on/off")),
    }
}
