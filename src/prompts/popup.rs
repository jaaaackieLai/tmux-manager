use crate::{Result, error::error};
use std::path::Path;
fn quote(value: &str) -> Result<String> {
    if value.chars().any(char::is_control) {
        return Err(error("binding 路徑不能包含控制字元"));
    }
    Ok(format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    ))
}
pub fn binding(binary: &Path, config: Option<&Path>) -> Result<String> {
    let binary = binary
        .to_str()
        .ok_or_else(|| error("binary 路徑必須為 UTF-8"))?;
    let config = config
        .map(|p| p.to_str().ok_or_else(|| error("config 路徑必須為 UTF-8")))
        .transpose()?;
    let config_arg = config
        .map(|p| quote(&p.replace('#', "##")).map(|q| format!(" --config-file {q}")))
        .transpose()?
        .unwrap_or_default();
    // display-popup 的 argv 不展開 format；run-shell -C 先以原 target 展開 tmux 指令。
    // -EE：只在成功結束時關閉，啟動錯誤才看得到。
    // -C 執行 tmux command，display-popup 多參數直接 exec binary，全程不拼 shell 指令。
    let popup = format!(
        "display-popup -EE -w 80% -h 80% {} prompts --target-pane #{{pane_id}} --socket #{{q:socket_path}}{config_arg}",
        quote(&binary.replace('#', "##"))?
    );
    Ok(format!("bind-key P run-shell -C {}\n", quote(&popup)?))
}
