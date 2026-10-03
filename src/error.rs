#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("設定格式錯誤：{0}")]
    Toml(#[from] toml::de::Error),
    #[error("無法序列化：{0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("HTTP 錯誤：{0}")]
    Http(#[from] reqwest::Error),
    #[error("資料已被其他視窗修改 (Conflict)；草稿已保留，請重新載入後合併")]
    Conflict,
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn error(message: impl Into<String>) -> Error {
    Error::Message(message.into())
}
