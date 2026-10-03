use crate::{
    Result,
    cli::{Cli, Commands},
    config::{Config, EnvMap, home_dir},
    distribution,
    error::error,
    prompts::{self, PromptStore, paste::PasteTarget},
    tmux::TmuxClient,
    ui::terminal,
};
use std::path::{Path, PathBuf};
fn default_prefix(env: &EnvMap) -> PathBuf {
    env.get("INSTALL_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir(env).join(".local"))
}
fn current_prefix(current: &Path) -> Result<PathBuf> {
    let parent = current
        .parent()
        .filter(|p| p.file_name().is_some_and(|s| s == "bin"))
        .and_then(Path::parent)
        .ok_or_else(|| {
            error("此 binary 尚未經 Rust installer 安裝；請使用 install --prefix PATH")
        })?;
    Ok(parent.into())
}
pub async fn run(cli: Cli) -> Result<()> {
    let env: EnvMap = std::env::vars().collect();
    let current = std::env::current_exe()?;
    if cli.update || matches!(cli.command, Some(Commands::Update)) {
        let report = distribution::update(&current).await?;
        println!(
            "{} {}",
            if report.updated {
                "已更新至"
            } else {
                "已是最新版本"
            },
            report.version
        );
        return Ok(());
    }
    if cli.uninstall || matches!(cli.command, Some(Commands::Uninstall { .. })) {
        let (prefix, purge) = match &cli.command {
            Some(Commands::Uninstall { prefix, purge }) => (prefix.clone(), *purge),
            _ => (None, false),
        };
        let prefix = match prefix {
            Some(prefix) => prefix,
            None => current_prefix(&current)?,
        };
        distribution::uninstall(&distribution::read_manifest(&prefix)?)?;
        if purge {
            distribution::purge_user_data(&Config::resolve_path(&cli.overrides, &env))?;
            println!("已移除 tmux-manager、設定與 prompts");
        } else {
            println!("已移除 tmux-manager；設定與 prompts 保留（uninstall --purge 可一併刪除）");
        }
        return Ok(());
    }
    if let Some(Commands::Install { prefix }) = &cli.command {
        let report = distribution::install(
            &current,
            &prefix.clone().unwrap_or_else(|| default_prefix(&env)),
        )?;
        println!("已安裝：{}", report.binary.display());
        if let Some(backup) = report.backup {
            println!("舊版備份：{}", backup.display());
        }
        return Ok(());
    }
    let path = Config::resolve_path(&cli.overrides, &env);
    if let Some(Commands::MigrateConfig {
        source,
        destination,
    }) = &cli.command
    {
        let report = crate::config::migrate_legacy(
            source,
            destination.as_deref().unwrap_or(&path),
            &home_dir(&env),
        )?;
        println!(
            "已匯入 {} 個設定至 {}；來源原檔保留",
            report.imported,
            report.destination.display()
        );
        return Ok(());
    }
    if let Some(Commands::Bindings { .. }) = &cli.command {
        print!("{}", prompts::popup::binding(&current, Some(&path))?);
        return Ok(());
    }
    if let Some(args) = &cli.config {
        if args.len() == 2 {
            Config::set(&path, &home_dir(&env), &args[0], &args[1])?;
            println!("{}={}", args[0], Config::read(&path)?.get(&args[0])?);
        } else {
            let config = Config::load(&cli.overrides, &env)?;
            if let Some(notice) = &config.notice {
                eprintln!("tmux-manager：{notice}");
            }
            if let Some(key) = args.first() {
                println!("{}", config.get(key)?);
            } else {
                for key in Config::keys() {
                    println!("{key}={}", config.get(key)?);
                }
            }
        }
        return Ok(());
    }
    let config = Config::load(&cli.overrides, &env)?;
    // 子程序（底部列、popup）可能在其他 cwd 執行：socket 一律用絕對路徑。
    let socket = cli
        .socket
        .or_else(|| TmuxClient::inherited_socket(&env))
        .map(|socket| std::path::absolute(&socket).unwrap_or(socket));
    let tmux = TmuxClient::new(socket);
    terminal::install_panic_hook();
    if cli.command.is_none()
        || matches!(
            cli.command,
            Some(Commands::Prompts {
                target_pane: Some(_)
            })
        )
    {
        tmux.check_version().await?;
    }
    match cli.command {
        Some(Commands::PromptDockSession { session }) => {
            prompts::dock_session::run(tmux, config.path, session).await
        }
        Some(Commands::PromptDock { initial_pane }) => {
            let _guard = terminal::TerminalGuard::enter()?;
            let mut terminal = terminal::terminal()?;
            prompts::dock::run(
                &mut terminal,
                &mut terminal::Input::new(),
                PromptStore::new(config.prompts_path()),
                tmux,
                initial_pane,
            )
            .await
        }
        Some(Commands::Prompts { target_pane }) => {
            let target = target_pane.map(|pane_id| PasteTarget {
                socket: tmux.socket.clone(),
                pane_id,
            });
            if target.is_some() && tmux.socket.is_none() {
                return Err(error(
                    "非 tmux 環境需 --socket PATH 明確指定原 server；可不帶 target 進入 prompt 管理模式",
                ));
            }
            let _guard = terminal::TerminalGuard::enter()?;
            let mut terminal = terminal::terminal()?;
            prompts::runtime::run(
                &mut terminal,
                &mut terminal::Input::new(),
                PromptStore::new(config.prompts_path()),
                tmux,
                target,
            )
            .await
        }
        None => {
            crate::app::runtime::run(
                tmux,
                config,
                env.get("ANTHROPIC_API_KEY").cloned(),
                env.contains_key("TMUX"),
            )
            .await
        }
        _ => Err(error("無效 CLI 組合")),
    }
}
