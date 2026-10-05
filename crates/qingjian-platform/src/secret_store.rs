//! 云密钥与配置分离；Windows 只落盘当前用户 DPAPI 密文。

use std::io;
use std::io::Write;
use std::path::Path;

pub fn save(directory: &Path, key: &str) -> io::Result<()> {
    let path = directory.join("cloud-key.dpapi");
    if key.trim().is_empty() {
        return match std::fs::remove_file(path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
    }
    #[cfg(windows)]
    {
        let encrypted = crate::windows_security::protect_secret(key.as_bytes())?;
        std::fs::create_dir_all(directory)?;
        qingjian_core::storage::write_atomic_private(&path, |writer| writer.write_all(&encrypted))
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "use environment credentials on this platform",
        ))
    }
}

pub fn load(directory: &Path) -> io::Result<Option<String>> {
    let path = directory.join("cloud-key.dpapi");
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    #[cfg(windows)]
    {
        String::from_utf8(crate::windows_security::unprotect_secret(&bytes)?)
            .map(Some)
            .map_err(|_| io::Error::other("invalid protected credential"))
    }
    #[cfg(not(windows))]
    {
        let _ = bytes;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows protected credential",
        ))
    }
}

/// 导出的配置只保留非秘密字段；不导出 .env 和 DPAPI 文件。
pub fn sanitized_config(source: &str) -> Result<String, toml_edit::TomlError> {
    let mut document: toml_edit::DocumentMut = source.parse()?;
    if let Some(table) = document
        .get_mut("predict")
        .and_then(toml_edit::Item::as_table_like_mut)
    {
        table.remove("api_key");
    }
    Ok(document.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn export_removes_secret_including_inline_comment() {
        let sanitized = super::sanitized_config(
            "[predict]\napi_key = 'dummy-secret' # dummy-secret\nenabled = false\n",
        )
        .unwrap();
        assert!(!sanitized.contains("dummy-secret"));
        assert!(sanitized.contains("enabled"));
        assert!(
            !super::sanitized_config("predict = { api_key = 'dummy-secret', enabled = false }\n")
                .unwrap()
                .contains("dummy-secret")
        );
    }
}
