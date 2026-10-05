use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write config {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    // TOML 错误自带原文片段；不保存源错误，避免非法配置中的密钥进入日志或错误链。
    #[error("invalid config {path}: check TOML syntax")]
    Parse { path: PathBuf },

    #[error("cannot edit config {path}: check TOML syntax")]
    Edit { path: PathBuf },
}
