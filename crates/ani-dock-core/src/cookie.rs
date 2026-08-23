use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use ts_rs::TS;

use crate::constant::COOKIE_FILE_PATH;

#[derive(Debug, Error)]
pub enum CookieError {
    #[error("文件操作失败：{desp}")]
    IO {
        desp: String,

        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Cookie(Arc<Mutex<String>>);

impl PartialEq for Cookie {
    fn eq(&self, other: &Self) -> bool {
        let cookie = self.0.lock().unwrap().to_owned();
        cookie == other.0.lock().unwrap().to_owned()
    }
}

impl Eq for Cookie {}

impl Display for Cookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.lock().unwrap().to_owned())
    }
}

impl Cookie {
    pub fn new<T: Into<String>>(cookie: T) -> Self {
        Self(Arc::new(Mutex::new(cookie.into())))
    }

    pub async fn set_and_write_cookie(
        &mut self,
        cookie: impl Into<String>,
    ) -> Result<(), CookieError> {
        self.set_cookie(cookie);
        self.write_cookie().await
    }

    pub fn set_cookie(&mut self, cookie: impl Into<String>) {
        *self.0.lock().unwrap() = cookie.into();
    }

    pub async fn write_cookie(&self) -> Result<(), CookieError> {
        if let Some(parent_path) = COOKIE_FILE_PATH.parent() {
            fs::create_dir_all(parent_path)
                .await
                .map_err(|source| CookieError::IO {
                    desp: "创建 cookie 文件父级目录失败".into(),
                    source,
                })?;
        }

        let cookie_string = self.to_string();
        fs::write(COOKIE_FILE_PATH.as_path(), cookie_string)
            .await
            .map_err(|source| CookieError::IO {
                desp: "写入 cookie 文件失败".into(),
                source,
            })?;

        Ok(())
    }

    pub async fn read_cookie() -> Result<Self, CookieError> {
        if !fs::try_exists(COOKIE_FILE_PATH.as_path())
            .await
            .map_err(|source| CookieError::IO {
                desp: "判断 cookie 文件是否存在错误".into(),
                source,
            })?
        {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(COOKIE_FILE_PATH.as_path())
            .await
            .map_err(|source| CookieError::IO {
                desp: "读取 cookie 文件发生错误".into(),
                source,
            })?;

        Ok(Self::new(
            contents.trim_end_matches(['\r', '\n']).to_string(),
        ))
    }
}

#[cfg(test)]
mod test {
    use std::{error::Error, sync::Mutex};

    use super::*;

    static COOKIE_FILE_LOCK: Mutex<()> = Mutex::new(());

    struct TestCookieFile {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TestCookieFile {
        fn new() -> Self {
            let lock = COOKIE_FILE_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            Self::remove_file();
            if let Some(parent) = COOKIE_FILE_PATH.parent() {
                std::fs::create_dir_all(parent)
                    .expect("test cookie parent directory should be created");
            }

            Self { _lock: lock }
        }

        fn remove_file() {
            let _ = std::fs::remove_file(COOKIE_FILE_PATH.as_path());
            let _ = std::fs::remove_dir_all(COOKIE_FILE_PATH.as_path());
        }
    }

    impl Drop for TestCookieFile {
        fn drop(&mut self) {
            Self::remove_file();
        }
    }

    #[test]
    fn cloned_cookies_share_in_memory_updates() {
        let cookie = Cookie::new("session=old");
        let mut cloned = cookie.clone();

        cloned.set_cookie("session=new");

        assert_eq!(cookie.to_string(), "session=new");
        assert_eq!(cookie, cloned);
    }

    #[tokio::test]
    async fn read_cookie_returns_default_when_file_is_missing() -> Result<(), Box<dyn Error>> {
        let _cookie_file = TestCookieFile::new();

        let cookie = Cookie::read_cookie().await?;

        assert_eq!(cookie, Cookie::default());
        assert_eq!(cookie.to_string(), "");

        Ok(())
    }

    #[tokio::test]
    async fn write_and_read_cookie_round_trip() -> Result<(), Box<dyn Error>> {
        let _cookie_file = TestCookieFile::new();
        let expected = Cookie::new("foo=bar; session=123");

        expected.write_cookie().await?;
        let actual = Cookie::read_cookie().await?;

        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), "foo=bar; session=123");

        Ok(())
    }

    #[tokio::test]
    async fn read_cookie_removes_trailing_line_endings() -> Result<(), Box<dyn Error>> {
        let _cookie_file = TestCookieFile::new();
        fs::write(COOKIE_FILE_PATH.as_path(), "foo=bar\r\n").await?;

        let cookie = Cookie::read_cookie().await?;

        assert_eq!(cookie.to_string(), "foo=bar");

        Ok(())
    }

    #[tokio::test]
    async fn cookie_file_operations_report_io_errors() {
        let _cookie_file = TestCookieFile::new();
        std::fs::create_dir(COOKIE_FILE_PATH.as_path())
            .expect("test cookie directory should be created");

        let read_error = Cookie::read_cookie()
            .await
            .expect_err("reading a directory as a cookie file should fail");
        let write_error = Cookie::new("foo=bar")
            .write_cookie()
            .await
            .expect_err("writing a cookie to a directory should fail");

        assert!(matches!(read_error, CookieError::IO { .. }));
        assert!(matches!(write_error, CookieError::IO { .. }));
    }
}
