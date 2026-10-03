use crate::{
    config::CliOverrides,
    tmux::{PaneId, SessionId},
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Debug, Parser)]
#[command(
    name = "tmux-manager",
    version,
    about = "tmux 工作階段管理與 Prompt Slots"
)]
pub struct Cli {
    #[command(flatten)]
    pub overrides: CliOverrides,
    #[arg(long, global = true)]
    pub socket: Option<PathBuf>,
    #[arg(long,num_args=0..=2)]
    pub config: Option<Vec<String>>,
    #[arg(long, requires = "config")]
    pub list: bool,
    #[arg(long,conflicts_with_all=["config","uninstall"])]
    pub update: bool,
    #[arg(long,conflicts_with_all=["config","update"])]
    pub uninstall: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}
fn pane_id(value: &str) -> std::result::Result<PaneId, String> {
    PaneId::parse(value).map_err(|e| e.to_string())
}
fn session_id(value: &str) -> std::result::Result<SessionId, String> {
    SessionId::parse(value).map_err(|e| e.to_string())
}
#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(hide = true)]
    PromptDock {
        #[arg(long, value_parser=pane_id)]
        initial_pane: PaneId,
    },
    #[command(hide = true)]
    PromptDockSession {
        #[arg(long, value_parser=session_id)]
        session: SessionId,
    },
    /// 管理 prompts；只有明確 target 才能貼上
    Prompts {
        #[arg(long,value_parser=pane_id)]
        target_pane: Option<PaneId>,
    },
    /// 輸出 prefix + P 的 tmux 設定（不自動寫入）
    Bindings {
        #[arg(long)]
        print: bool,
    },
    /// 匯入允許的簡單 Bash assignment，不執行 shell
    MigrateConfig {
        source: PathBuf,
        #[arg(long)]
        destination: Option<PathBuf>,
    },
    /// 安裝目前 binary 至指定 prefix
    Install {
        #[arg(long)]
        prefix: Option<PathBuf>,
    },
    /// 更新已由 Rust installer 管理的 binary
    Update,
    /// 依 manifest 移除 binary、備份與舊 Bash 版檔案；預設保留 config/prompts
    Uninstall {
        #[arg(long)]
        prefix: Option<PathBuf>,
        /// 一併刪除 config 與 prompts
        #[arg(long)]
        purge: bool,
    },
}
