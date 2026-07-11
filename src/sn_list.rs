use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{config::DownloadMode, constant::SN_LIST_FILE_PATH, error::SnListError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnDetail {
    /// sn 码
    pub code: String,
    /// 下载模式，如空则使用配置中的默认下载模式
    pub mode: Option<DownloadMode>,
    // TODO impl rename
    // TODO impl bangumi custom aggregation
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SnList(pub Vec<SnDetail>);

/// SN 列表的 TOML 序列化桥接类型。
///
/// TOML 不支持顶层数组，因此需要用一个表包裹 `SnList` 中的数组；该类型不应暴露给上层使用。
#[derive(Serialize, Deserialize)]
struct TomlSnList {
    #[serde(default)]
    entries: Vec<SnDetail>,
}

impl From<TomlSnList> for SnList {
    fn from(value: TomlSnList) -> Self {
        Self(value.entries)
    }
}

impl From<&SnList> for TomlSnList {
    fn from(value: &SnList) -> Self {
        Self {
            entries: value.0.clone(),
        }
    }
}

impl SnList {
    pub async fn write_sn_list(&self) -> Result<(), SnListError> {
        if let Some(parent_path) = SN_LIST_FILE_PATH.parent() {
            fs::create_dir_all(parent_path).await?
        }

        let toml_sn_list = TomlSnList::from(self);
        fs::write(
            SN_LIST_FILE_PATH.as_path(),
            toml::to_string_pretty(&toml_sn_list)?,
        )
        .await?;

        Ok(())
    }

    pub async fn read_sn_list() -> Result<Self, SnListError> {
        if !fs::try_exists(SN_LIST_FILE_PATH.as_path()).await? {
            return Ok(Default::default());
        }

        let contents = fs::read_to_string(SN_LIST_FILE_PATH.as_path()).await?;
        let toml_sn_list = toml::from_str::<TomlSnList>(&contents)?;

        Ok(toml_sn_list.into())
    }
}

impl IntoIterator for SnList {
    type Item = SnDetail;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a SnList {
    type Item = &'a SnDetail;
    type IntoIter = std::slice::Iter<'a, SnDetail>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut SnList {
    type Item = &'a mut SnDetail;
    type IntoIter = std::slice::IterMut<'a, SnDetail>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

#[cfg(test)]
mod test {
    use std::{error::Error, sync::Mutex};

    use super::*;

    static SN_LIST_FILE_LOCK: Mutex<()> = Mutex::new(());

    struct TestSnListFile {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TestSnListFile {
        fn new() -> Self {
            let lock = SN_LIST_FILE_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            Self::remove_file();
            if let Some(parent) = SN_LIST_FILE_PATH.parent() {
                std::fs::create_dir_all(parent)
                    .expect("test sn list parent directory should be created");
            }

            Self { _lock: lock }
        }

        fn remove_file() {
            let _ = std::fs::remove_file(SN_LIST_FILE_PATH.as_path());
            let _ = std::fs::remove_dir_all(SN_LIST_FILE_PATH.as_path());
        }
    }

    impl Drop for TestSnListFile {
        fn drop(&mut self) {
            Self::remove_file();
        }
    }

    #[test]
    fn empty_toml_uses_empty_sn_list() -> Result<(), Box<dyn Error>> {
        let sn_list: TomlSnList = toml::from_str("")?;

        assert_eq!(SnList::from(sn_list), SnList::default());

        Ok(())
    }

    #[tokio::test]
    async fn read_sn_list_returns_default_when_file_is_missing() -> Result<(), Box<dyn Error>> {
        let _sn_list_file = TestSnListFile::new();

        let sn_list = SnList::read_sn_list().await?;

        assert_eq!(sn_list, SnList::default());
        assert!(!SN_LIST_FILE_PATH.exists());

        Ok(())
    }

    #[tokio::test]
    async fn write_and_read_sn_list_round_trip() -> Result<(), Box<dyn Error>> {
        let _sn_list_file = TestSnListFile::new();
        let expected = SnList(vec![
            SnDetail {
                code: "12345".to_string(),
                mode: None,
            },
            SnDetail {
                code: "67890".to_string(),
                mode: Some(DownloadMode::All),
            },
        ]);

        expected.write_sn_list().await?;
        let actual = SnList::read_sn_list().await?;

        assert_eq!(actual, expected);

        Ok(())
    }

    #[tokio::test]
    async fn read_sn_list_reports_invalid_toml() -> Result<(), Box<dyn Error>> {
        let _sn_list_file = TestSnListFile::new();
        fs::write(SN_LIST_FILE_PATH.as_path(), "sn = [").await?;

        let error = SnList::read_sn_list()
            .await
            .expect_err("invalid TOML should fail to parse");

        assert!(matches!(error, SnListError::TomlDe(_)));

        Ok(())
    }

    #[tokio::test]
    async fn sn_list_file_operations_report_io_errors() {
        let _sn_list_file = TestSnListFile::new();
        std::fs::create_dir(SN_LIST_FILE_PATH.as_path())
            .expect("test sn list directory should be created");

        let read_error = SnList::read_sn_list()
            .await
            .expect_err("reading a directory as an sn list file should fail");
        let write_error = SnList::default()
            .write_sn_list()
            .await
            .expect_err("writing an sn list to a directory should fail");

        assert!(matches!(read_error, SnListError::IO(_)));
        assert!(matches!(write_error, SnListError::IO(_)));
    }
}
