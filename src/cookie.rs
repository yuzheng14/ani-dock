use tokio::fs;

use crate::{constant::COOKIE_FILE_PATH, error::CookieError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie(pub String);

impl Cookie {
    pub fn new<T: Into<String>>(cookie: T) -> Self {
        Self(cookie.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub async fn write_cookie(&self) -> Result<(), CookieError> {
        if let Some(parent_path) = COOKIE_FILE_PATH.parent() {
            fs::create_dir_all(parent_path).await?;
        }

        fs::write(COOKIE_FILE_PATH.as_path(), &self.0).await?;

        Ok(())
    }

    pub async fn read_cookie() -> Result<Self, CookieError> {
        match fs::try_exists(COOKIE_FILE_PATH.as_path()).await? {
            true => {
                let contents = fs::read_to_string(COOKIE_FILE_PATH.as_path()).await?;

                Ok(Cookie(contents.trim_end_matches(['\r', '\n']).to_string()))
            }
            false => Err(CookieError::NotFound),
        }
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

    #[tokio::test]
    async fn read_cookie_reports_missing_file() {
        let _cookie_file = TestCookieFile::new();

        let error = Cookie::read_cookie()
            .await
            .expect_err("a missing cookie file should fail");

        assert!(matches!(error, CookieError::NotFound));
    }

    #[tokio::test]
    async fn write_and_read_cookie_round_trip() -> Result<(), Box<dyn Error>> {
        let _cookie_file = TestCookieFile::new();
        let expected = Cookie::new("foo=bar; session=123");

        expected.write_cookie().await?;
        let actual = Cookie::read_cookie().await?;

        assert_eq!(actual, expected);
        assert_eq!(actual.as_str(), "foo=bar; session=123");

        Ok(())
    }

    #[tokio::test]
    async fn read_cookie_removes_trailing_line_endings() -> Result<(), Box<dyn Error>> {
        let _cookie_file = TestCookieFile::new();
        fs::write(COOKIE_FILE_PATH.as_path(), "foo=bar\r\n").await?;

        let cookie = Cookie::read_cookie().await?;

        assert_eq!(cookie.into_inner(), "foo=bar");

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

        assert!(matches!(read_error, CookieError::IO(_)));
        assert!(matches!(write_error, CookieError::IO(_)));
    }
}
