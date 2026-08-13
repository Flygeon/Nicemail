//! 统一错误类型。所有命令最终以 `Result<T, String>` 返回,
//! 通过 `impl From<Error> for String` 自动转换。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("数据库错误: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("IMAP 错误: {0}")]
    Imap(#[from] imap::Error),
    #[error("SMTP 错误: {0}")]
    Smtp(#[from] lettre::transport::smtp::Error),
    #[error("邮件解析错误: {0}")]
    Mime(String),
    #[error("密钥环错误: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("OAuth 错误: {0}")]
    OAuth(String),
    #[error("配置错误: {0}")]
    Config(String),
    #[error("无效输入: {0}")]
    InvalidInput(String),
    #[error("TLS 错误: {0}")]
    Tls(#[from] native_tls::Error),
}

impl From<Error> for String {
    fn from(e: Error) -> Self {
        e.to_string()
    }
}
