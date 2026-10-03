use std::collections::VecDeque;
use std::sync::Mutex;
use tmux_manager::Result;
use tmux_manager::tmux::command::{BoxFuture, CommandOutput, CommandRequest, Runner};
#[derive(Default)]
pub struct FakeRunner {
    pub calls: Mutex<Vec<CommandRequest>>,
    outputs: Mutex<VecDeque<CommandOutput>>,
}
impl FakeRunner {
    pub fn new(outputs: Vec<(i32, &str, &str)>) -> Self {
        Self {
            calls: Mutex::default(),
            outputs: Mutex::new(
                outputs
                    .into_iter()
                    .map(|(status, out, err)| CommandOutput {
                        status,
                        stdout: out.as_bytes().to_vec(),
                        stderr: err.into(),
                    })
                    .collect(),
            ),
        }
    }
}
impl Runner for FakeRunner {
    fn run(&self, request: CommandRequest) -> BoxFuture<'_, Result<CommandOutput>> {
        let section = request.args.windows(3).find_map(|args| {
            (args[0] == "display-message" && args[1] == "-p").then(|| args[2].clone())
        });
        self.calls.lock().unwrap().push(request);
        let mut output = self
            .outputs
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected tmux invocation");
        // batch fixture 的 placeholders 取自此次指令的 nonce，包含 tmux 的 escape 形式。
        if let Some(section) = section {
            output.stdout = String::from_utf8(output.stdout)
                .unwrap()
                .replace("<BATCH_SECTION>", &section)
                .replace(
                    "<BATCH_PRINTED_SECTION>",
                    &section.replace('\u{1}', "\\001"),
                )
                .into_bytes();
        }
        Box::pin(std::future::ready(Ok(output)))
    }
}
