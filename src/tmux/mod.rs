pub mod capabilities;
pub mod command;
pub mod dock;
pub mod model;
pub mod session;
use crate::{Result, error::error};
use command::{CommandOutput, CommandRequest, ProcessRunner, Runner};
pub use model::*;
pub use session::Snapshot;
use std::{path::PathBuf, sync::Arc};
#[derive(Clone)]
pub struct TmuxClient {
    pub socket: Option<PathBuf>,
    runner: Arc<dyn Runner>,
}
impl TmuxClient {
    pub fn new(socket: Option<PathBuf>) -> Self {
        Self::with_runner(socket, Arc::new(ProcessRunner))
    }
    pub fn with_runner(socket: Option<PathBuf>, runner: Arc<dyn Runner>) -> Self {
        Self { socket, runner }
    }
    pub async fn run(
        &self,
        args: &[&str],
        stdin: &[u8],
        interactive: bool,
    ) -> Result<CommandOutput> {
        self.runner
            .run(CommandRequest {
                socket: self.socket.clone(),
                args: args.iter().map(|s| (*s).into()).collect(),
                stdin: stdin.into(),
                interactive,
            })
            .await
    }
    pub async fn checked(&self, args: &[&str]) -> Result<String> {
        self.checked_with_stdin(args, &[]).await
    }
    pub async fn checked_with_stdin(&self, args: &[&str], stdin: &[u8]) -> Result<String> {
        let output = self.run(args, stdin, false).await?;
        if output.status != 0 {
            return Err(error(format!("tmux：{}", output.stderr.trim())));
        }
        String::from_utf8(output.stdout).map_err(|_| error("tmux 輸出不是有效 UTF-8"))
    }
    pub fn inherited_socket(env: &std::collections::BTreeMap<String, String>) -> Option<PathBuf> {
        env.get("TMUX")
            .and_then(|s| s.split(',').next())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
    }
}
