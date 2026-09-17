pub mod activation;
pub mod config;
pub mod connections;
pub mod gateway;
pub mod profiles;
pub mod protocol;
pub mod providers;
pub mod store;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("配置被其他程序修改，本次操作未完成。请停止其他配置操作后重试。")]
    Conflict,
    #[error("本地文件操作失败，请检查目录权限。")]
    Io(#[from] std::io::Error),
    #[error("本地数据格式无效。")]
    Json(#[from] serde_json::Error),
    #[error("本地数据库操作失败。")]
    Sql(#[from] rusqlite::Error),
}
pub fn message(s: impl Into<String>) -> Error {
    Error::Message(s.into())
}
