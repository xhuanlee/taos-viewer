use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("TDengine: {0}")]
    Taos(#[from] taos::Error),
    #[error("连接不存在或尚未建立，请先连接服务器")]
    NotConnected,
    #[error("SQL 执行失败 [{sql}]: {source}")]
    SqlFailed { sql: String, source: taos::Error },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
