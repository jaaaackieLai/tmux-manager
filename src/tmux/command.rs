use crate::{Result, error::error};
use std::{future::Future, path::PathBuf, pin::Pin, process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command};
#[derive(Clone, Debug)]
pub struct CommandRequest {
    pub socket: Option<PathBuf>,
    pub args: Vec<String>,
    pub stdin: Vec<u8>,
    pub interactive: bool,
}
#[derive(Debug)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub trait Runner: Send + Sync {
    fn run(&self, request: CommandRequest) -> BoxFuture<'_, Result<CommandOutput>>;
}
#[derive(Default)]
pub struct ProcessRunner;
fn spawn_error(e: std::io::Error) -> crate::error::Error {
    error(format!("無法執行 tmux：{e}；請安裝 tmux 3.3+"))
}
impl Runner for ProcessRunner {
    fn run(&self, request: CommandRequest) -> BoxFuture<'_, Result<CommandOutput>> {
        Box::pin(async move {
            let mut command = Command::new("tmux");
            // 輸出交給本工具解析：非 UTF-8 locale 下 tmux 會把 tab 與中文換成 `_`。
            // attach 畫面由使用者終端機顯示，維持 tmux 依 locale 判斷。
            if !request.interactive {
                command.arg("-u");
            }
            if let Some(socket) = &request.socket {
                command.arg("-S").arg(socket);
            }
            command.args(&request.args);
            if request.interactive {
                let status = command.status().await.map_err(spawn_error)?;
                return Ok(CommandOutput {
                    status: status.code().unwrap_or(1),
                    stdout: Vec::new(),
                    stderr: String::new(),
                });
            }
            let mut child = command
                .stdin(if request.stdin.is_empty() {
                    Stdio::null()
                } else {
                    Stdio::piped()
                })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .map_err(spawn_error)?;
            let mut input = child.stdin.take();
            let write = async {
                if let Some(input) = input.as_mut() {
                    let _ = input.write_all(&request.stdin).await;
                }
                drop(input.take());
            };
            // 逾時時 future 被丟棄，kill_on_drop 會終止 tmux。
            let (_, output) = tokio::time::timeout(Duration::from_secs(2), async {
                tokio::join!(write, child.wait_with_output())
            })
            .await
            .map_err(|_| error("tmux 指令超時（2 秒）"))?;
            let output = output?;
            Ok(CommandOutput {
                status: output.status.code().unwrap_or(1),
                stdout: output.stdout,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        })
    }
}
