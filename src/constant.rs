use std::{path::PathBuf, str, sync::LazyLock};

pub static APP_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    directories::UserDirs::new()
        .expect("Could not get config dir")
        .home_dir()
        .to_owned()
        .join(".ani-dock")
});

#[cfg(not(test))]
pub static CONFIG_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("config.toml"));
#[cfg(test)]
pub static CONFIG_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("config.test.toml"));

#[cfg(not(test))]
pub static SN_LIST_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("sn_list.toml"));
#[cfg(test)]
pub static SN_LIST_FILE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| APP_DIR.join("sn_list.test.toml"));

#[cfg(not(test))]
pub static DB_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("data.db"));
#[cfg(test)]
pub static DB_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("data.test.db"));

#[cfg(not(test))]
pub static COOKIE_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("cookie.txt"));
#[cfg(test)]
pub static COOKIE_FILE_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("cookie.test.txt"));

#[cfg(not(test))]
pub static BANGUMI_DIR_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("bangumi"));
#[cfg(test)]
pub static BANGUMI_DIR_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("bangumi-test"));

#[cfg(not(test))]
pub static TMP_DIR_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("tmp"));
#[cfg(test)]
pub static TMP_DIR_PATH: LazyLock<PathBuf> = LazyLock::new(|| APP_DIR.join("tmp-test"));

/// 动画疯域名 ORIGIN
///
/// https://ani.gamer.com.tw
pub const ORIGIN: &str = "https://ani.gamer.com.tw";
/// 动画疯 api 域名 ORIGIN
///
/// https://api.gamer.com.tw
pub const API_ORIGIN: &str = "https://api.gamer.com.tw";

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::anyhow;
    use std::error::Error;

    fn get_home_dir() -> Result<PathBuf, anyhow::Error> {
        std::env::home_dir().ok_or(anyhow!("get home dir error"))
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn app_dir_on_macos() -> Result<(), Box<dyn Error>> {
        assert_eq!(APP_DIR.as_path(), get_home_dir()?.join(".ani-dock"));

        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn config_file_path_on_macos() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            CONFIG_FILE_PATH.as_path(),
            get_home_dir()?.join(".ani-dock").join("config.test.toml")
        );

        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sn_list_file_path_on_macos() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            SN_LIST_FILE_PATH.as_path(),
            get_home_dir()?.join(".ani-dock").join("sn_list.test.toml")
        );

        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn db_file_path_on_macos() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            DB_FILE_PATH.as_path(),
            get_home_dir()?.join(".ani-dock").join("data.test.db")
        );

        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn cookie_file_path_on_macos() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            COOKIE_FILE_PATH.as_path(),
            get_home_dir()?.join(".ani-dock").join("cookie.test.txt")
        );

        Ok(())
    }
}
