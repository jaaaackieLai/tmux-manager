use super::*;
#[derive(Debug)]
pub struct MigrationReport {
    pub imported: usize,
    pub destination: PathBuf,
}
pub fn migrate_legacy(source: &Path, destination: &Path, home: &Path) -> Result<MigrationReport> {
    if destination.exists() {
        return Err(error("遷移目的檔已存在，保留原檔"));
    }
    let text = std::fs::read_to_string(source)?;
    let mut config = Config::default();
    let mut imported = 0;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parse = || -> Result<(String, String)> {
            let (key, raw) = line
                .split_once('=')
                .ok_or_else(|| error("只接受簡單 assignment"))?;
            let key = key
                .strip_prefix("TMUX_MANAGER_")
                .ok_or_else(|| error("不允許的設定鍵"))?;
            if !Config::keys().contains(&key) {
                return Err(error("不允許的設定鍵"));
            }
            let raw = raw.trim();
            if raw.contains(['`', '\\']) {
                return Err(error("拒絕 shell 指令、命令替換或跳脫語法"));
            }
            let quote = raw.chars().next().filter(|c| matches!(c, '\'' | '"'));
            let (mut value, expand) = match quote {
                Some(quote) if raw.len() >= 2 && raw.ends_with(quote) => {
                    let value = &raw[1..raw.len() - 1];
                    if value.contains(quote) {
                        return Err(error("引號格式錯誤"));
                    }
                    // 單引號內完全是字面值；雙引號內只展開 $HOME。
                    (value.to_string(), quote == '"')
                }
                _ => {
                    if raw.chars().any(char::is_whitespace) || raw.contains(['\'', '"']) {
                        return Err(error("只接受單一值或完整引號"));
                    }
                    // 雙引號內這些符號是字面值；未加引號時 Bash 會當成指令語法。
                    if raw.contains([';', '|', '&', '<', '>', '(', ')']) {
                        return Err(error("拒絕 shell 指令、命令替換或跳脫語法"));
                    }
                    (raw.to_string(), true)
                }
            };
            if expand {
                let home = home.to_string_lossy();
                let mut expanded = String::new();
                let mut rest = value.as_str();
                while let Some(index) = rest.find('$') {
                    expanded.push_str(&rest[..index]);
                    let variable = &rest[index..];
                    let length = if variable.starts_with("${HOME}") {
                        7
                    } else if variable.starts_with("$HOME")
                        && !variable[5..]
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        5
                    } else {
                        return Err(error("只允許 $HOME 或 ${HOME} 展開"));
                    };
                    expanded.push_str(&home);
                    rest = &variable[length..];
                }
                expanded.push_str(rest);
                value = expanded;
            }
            Ok((key.into(), value))
        };
        let (key, value) = parse().map_err(|e| error(format!("第 {} 行：{e}", index + 1)))?;
        config
            .assign(&key, &value)
            .map_err(|e| error(format!("第 {} 行：{e}", index + 1)))?;
        imported += 1;
    }
    storage::atomic_write(
        destination,
        toml::to_string_pretty(&config)?.as_bytes(),
        false,
    )?;
    Ok(MigrationReport {
        imported,
        destination: destination.into(),
    })
}
