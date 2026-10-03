use clap::Parser;
#[tokio::main]
async fn main() {
    if let Err(error) = tmux_manager::entry::run(tmux_manager::cli::Cli::parse()).await {
        eprintln!("tmux-manager：{error}");
        std::process::exit(1);
    }
}
