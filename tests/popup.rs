use clap::Parser;
use tmux_manager::{
    cli::{Cli, Commands},
    prompts::popup::binding,
};
#[test]
fn explicit_target_survives_popup_pane_environment() {
    let cli = Cli::try_parse_from([
        "tmux-manager",
        "prompts",
        "--target-pane",
        "%12",
        "--socket",
        "/tmp/original",
    ])
    .unwrap();
    let Some(Commands::Prompts { target_pane, .. }) = cli.command else {
        panic!("prompts command")
    };
    assert_eq!(target_pane.unwrap().as_str(), "%12");
    let untargeted = Cli::try_parse_from(["tmux-manager", "prompts"]).unwrap();
    assert!(matches!(
        untargeted.command,
        Some(Commands::Prompts {
            target_pane: None,
            ..
        })
    ));
    assert!(Cli::try_parse_from(["tmux-manager", "prompts", "--target-pane", "active"]).is_err());
}
#[test]
fn binding_captures_original_pane_and_socket_with_direct_arguments() {
    let result = binding(
        std::path::Path::new("/tmp/path with spaces/tmux-manager"),
        None,
    )
    .unwrap();
    assert!(result.contains("bind-key P run-shell -C"));
    assert!(result.contains("/tmp/path with spaces/tmux-manager"));
    assert!(result.contains("--target-pane #{pane_id} --socket #{q:socket_path}"));
    assert!(!result.contains("sh -c"));
}
#[test]
fn cli_config_keeps_empty_value_and_list_mode() {
    let cli = Cli::try_parse_from(["tmux-manager", "--config", "NEW_DEFAULT_CMD", ""]).unwrap();
    assert_eq!(cli.config.unwrap(), ["NEW_DEFAULT_CMD", ""]);
    let cli = Cli::try_parse_from(["tmux-manager", "--config", "--list"]).unwrap();
    assert!(cli.list);
}
#[test]
fn popup_stays_open_when_prompts_exit_with_an_error() {
    let result = binding(std::path::Path::new("/bin/tmux-manager"), None).unwrap();
    assert!(result.contains("display-popup -EE "), "{result}");
}
