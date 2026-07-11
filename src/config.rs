use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::{constant::CONFIG_FILE_PATH, error::ConfigError};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum DownloadResolution {
    /// 360p
    #[serde(rename = "360")]
    P360,
    /// 480p
    #[serde(rename = "480")]
    P480,
    /// 540p
    #[serde(rename = "540")]
    P540,
    /// 576p
    #[serde(rename = "576")]
    P576,
    /// 720p
    #[serde(rename = "720")]
    P720,
    /// 1080p
    #[serde(rename = "1080")]
    #[default]
    P1080,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadMode {
    /// 仅下载最新一集
    #[default]
    Latest,
    /// 下载所有集数
    All,
    /// 下载最近上传的一集
    LargestSn,
}

// #[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
// #[serde(rename_all = "lowercase")]
// pub enum VideoPackageExtension {
//     /// ts
//     Ts,
//     /// mov
//     Mov,
//     /// mkv
//     Mkv,
//     /// mp4
//     #[default]
//     Mp4,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct FtpSettings {
//     /// FTP Server IP
//     pub server: String,
//     /// 端口
//     pub port: String,
//     /// 使用者名
//     pub user: String,
//     /// 密碼
//     pub pwd: String,
//     /// 是否是 FTP over TLS
//     pub tls: bool,
//     /// 登陸後首先進入的目錄
//     pub cwd: String,
//     /// 是否顯示細節錯誤信息
//     pub show_error_detail: bool,
//     /// 最大重傳數, 支援續傳
//     pub max_retry_num: u32,
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigVersion {
    /// 主版本号，主版本号变更时，配置文件将无法兼容旧版本
    pub major: u32,
    /// 次版本号，次版本号变更时，配置文件将可以兼容旧版本
    pub minor: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalConfig {
    /// 配置文件版本
    pub config_version: ConfigVersion,
}

impl Default for InternalConfig {
    fn default() -> Self {
        Self {
            config_version: ConfigVersion { major: 1, minor: 0 },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    /// /// 下载存放目录，动画将会以番剧为单位分文件夹存放，默认为 `bangumi`。
    /// /// 相对路径时为程序所在目录的 `data` 文件夹子目录，绝对路径时为绝对路径。
    /// /// 如文件夹不存在，将会自动创建。
    /// pub bangumi_dir: String,
    /// /// 临时目录位置，v9.0 开始下载中文件将会放在这里，完成后再转移至番剧目录，留空默认为在程序所在目录的 temp 文件夹下，默认为 `temp`。
    /// /// 相对路径时为程序所在目录的 `data` 文件夹子目录，绝对路径时为绝对路径。
    /// /// 如文件夹不存在，将会自动创建。
    /// pub temp_dir: String,
    // 不开放此配置，默认以番剧为单位创建文件夹
    // /// 控制是否建立番劇資料夾
    // pub classify_bangumi: bool,
    // 不开放此配置，默认以季度为单位创建文件夹
    // /// 控制是否建立季度子目錄
    // #[serde(default)]
    // pub classify_season: bool,
    /// /// 番剧更新检查频率，单位为分钟，默认为 30 分钟。
    /// pub check_frequency: u64,
    /// /// 下载冷却时间(秒)，默认为 20 分钟。
    /// pub download_cd: u64,
    /// /// 每个番剧更新检查冷却实现（秒），默认为 5 秒。
    /// pub parse_sn_cd: u64,
    /// 下载选取清晰度，若该清晰度不存在将会选取最近可用清晰度，可选 360 480 540 576 720 1080，默认为 1080。
    pub download_resolution: DownloadResolution,
    /// 是否锁定清晰度，如果指定清晰度不存在，则放弃下载
    pub lock_resolution: bool,
    /// 是否只使用 VIP 账号下载
    pub only_use_vip: bool,
    /// 默认下载模式，可选 latest 和 all，默认为 latest。
    pub default_download_mode: DownloadMode,
    /// 是否优先移动文件，而不是复制文件。如果跨设备/文件系统移动，可以设置为 false，
    /// 文件将会先复制到目标文件夹，再删除源文件。避免移动失败的问题，默认为 true。
    pub prefer_move: bool,
    /// 最大并发下载数，最高为 5，超过将重置为 5。安全起见，默认值为 1。
    pub multi_thread: u32,
    /// /// ftp 上传最大并发数，最高为 3，超过将重置为 3
    /// pub multi_upload: u32,
    /// /// 分段下载模式，速度更快，容错率更高
    /// pub segment_download_mode: bool,
    /// 每个视频最大并发下载分段数，仅在 `segment_download_mode` 为 true 时有效，最高为 5，超过将重置为 5。
    /// 默认值为 2。
    pub multi_downloading_segment: u32,
    /// 在分段下载模式时有效，每个分段最大重试次数，-1 为无限重试。默认值为 8。
    pub segment_max_retry: i32,
    /// /// 是否在视频文件名中添加番剧名，格式举例: [番剧名]。
    /// /// 如果为 false，则只有集数。默认值为 true。
    /// pub add_bangumi_name_to_video_filename: bool,
    /// /// 是否在视频文件名中添加清晰度，格式举例: [1080P]。默认值为 true。
    /// pub add_resolution_to_video_filename: bool,
    /// /// 视频文件名自定义前缀，默认值为 【動畫瘋】。
    /// pub customized_video_filename_prefix: String,
    /// /// 视频文件名中番剧名的后缀，集数之前。默认值为空。
    /// pub customized_bangumi_name_suffix: String,
    /// /// 视频文件名后缀。默认值为空。
    /// pub customized_video_filename_suffix: String,
    /// /// 视频文件扩展名，ts, mov, mkv 经过测试可以使用，但 flv 不支持，非 mp4 扩展名 faststart_movflags 将强制为 false。默认值为 mp4。
    /// pub video_filename_extension: VideoPackageExtension,
    /// /// 剧集名补零，填写补足位数，例: 填写 2 剧集名称为 01，填写 3 剧集名称为 001。默认值为 1。
    /// pub zerofill: u32,
    /// 请求UA，需要和获取cookie的浏览器相同
    pub ua: String,
    /// /// 代理开关
    /// pub use_proxy: bool,
    /// 代理配置
    pub proxy: Option<String>,
    /// /// 为 true 时，对 Akamai CDN 的请求不走代理
    /// pub no_proxy_akamai: bool,
    // TODO 暂不实现
    // /// 上传功能开关
    // pub upload_to_server: bool,
    // /// FTP配置
    // pub ftp: FtpSettings,
    /// /// 命令行模式使用 -u 参数有效, 在命令行模式下完成所有任务后执行的命令。默认值为空。
    /// pub user_command: String,
    // FIXME 酷 Q 似乎已经凉了，先观察观察
    // /// 酷 Q 推送（README 中原 `coolq_notify` 與 `coolq_settings`）
    // #[serde(default)]
    // pub cool_q: CoolQ,
    // TODO 暂不实现
    // /// 适配 PLEX 命名规则。默认值为 false。
    // pub plex_naming: bool,
    /// /// 是否将视频 metadata 前置, 启用此功能时在线观看会更快播放, 仅在 video_filename_extension 为 mp4 时有效。默认值为 false。
    /// pub faststart_movflags: bool,
    /// /// 是否添加音轨标签，只有分段下载模式有效。默认值为 false。
    /// pub audio_language: bool,
    /// 使用移动端API进行视频解析。默认值为 false。
    pub use_mobile_api: bool,
    /// 是否下载弹幕(已包含动画疯内置的关键字过滤)。默认值为 false。
    pub danmu: bool,
    // TODO 暂不支持正则，英文区分大小写 （(支持python的正则表达式、英文不区分大小写)）
    /// 额外过滤弹幕关键字。默认值为空。
    pub danmu_ban_words: Vec<String>,
    // TODO 暂不实现
    // /// 是否检查更新。默认值为 true。
    // pub check_latest_version: bool,
    /// /// 是否在检查更新时读取 `sn_list.txt`, 开启后对 `sn_list.txt` 的更改将会在下次检查更新时生效而不用重启程序。默认值为 true。
    /// pub read_sn_list_when_checking_update: bool,
    /// /// 是否在检查更新时读取配置文件, 开启后对配置文件的更改将会在下次检查时更新生效而不用重启程序。默认值为 true。
    /// pub read_config_when_checking_update: bool,
    /// 非VIP广告等待时间, 如果等待时间不足, 程序会自行追加时间 (最大20秒)。默认值为 25。
    pub ads_time: u32,
    /// 使用移动端 API 解析的广告等待时间。默认值为 25。
    pub mobile_ads_time: u32,
    // TODO agm_server 实现后开放
    // /// Web 控制台開關
    // pub use_dashboard: bool,
    // /// Web控制面板配置
    // pub dashboard: Dashboard,
    /// 是否記錄日志, 一天一個日志
    pub save_logs: bool,
    /// 日志保留数量, 正整数值, 必须大于等于 1, 默认值为 7。
    pub quantity_of_logs: u32,
    /// 内部配置，请勿直接修改
    pub internal: InternalConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_resolution: Default::default(),
            lock_resolution: Default::default(),
            only_use_vip: Default::default(),
            default_download_mode: Default::default(),
            prefer_move: true,
            multi_thread: 1,
            multi_downloading_segment: 2,
            segment_max_retry: 8,
            ua: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0".to_string(),
            use_mobile_api: Default::default(),
            danmu: Default::default(),
            danmu_ban_words: Default::default(),
            ads_time: 25,
            mobile_ads_time: 25,
            proxy: Default::default(),
            save_logs: true,
            quantity_of_logs: 7,
            internal: Default::default(),
        }
    }
}

impl Config {
    pub async fn read_config() -> Result<Self, ConfigError> {
        if !fs::try_exists(CONFIG_FILE_PATH.as_path()).await? {
            Config::default().write_config().await?
        }

        let contents = fs::read_to_string(CONFIG_FILE_PATH.as_path()).await?;

        let mut config = toml::from_str::<Config>(&contents)?;
        config.normalize();

        Ok(config)
    }

    pub async fn write_config(&self) -> Result<(), ConfigError> {
        if let Some(parent) = CONFIG_FILE_PATH.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(CONFIG_FILE_PATH.as_path(), toml::to_string_pretty(&self)?).await?;

        Ok(())
    }

    fn normalize(&mut self) {
        self.multi_thread = self.multi_thread.clamp(1, 5);
        self.multi_downloading_segment = self.multi_downloading_segment.clamp(1, 5);

        if self.quantity_of_logs < 1 {
            self.quantity_of_logs = 1
        }
    }
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct Dashboard {
//     /// 監聽地址, 如果需要允許外部訪問, 請填寫 "0.0.0.0"
//     pub host: String,
//     /// 監聽端口
//     pub port: u16,
//     /// 是否開啓SSL, 證書保存在 Dashboard/sslkey, 如果有需要可以自行替換證書
//     pub ssl: bool,
//     /// 是否使用 BasicAuth 進行認證, 注意, 用戶密碼是明文傳輸的, 如有需要建議同時啓用 SSL
//     pub basic_auth: bool,
//     /// BasicAuth 用戶名
//     pub username: String,
//     /// BasicAuth 密碼
//     pub password: String,
// }

#[cfg(test)]
mod test {
    use std::{error::Error, sync::Mutex};

    use super::*;

    static CONFIG_FILE_LOCK: Mutex<()> = Mutex::new(());

    struct TestConfigFile {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TestConfigFile {
        fn new() -> Self {
            let lock = CONFIG_FILE_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            Self::remove_config();
            if let Some(parent) = CONFIG_FILE_PATH.parent() {
                std::fs::create_dir_all(parent)
                    .expect("test config parent directory should be created");
            }

            Self { _lock: lock }
        }

        fn remove_config() {
            let _ = std::fs::remove_file(CONFIG_FILE_PATH.as_path());
            let _ = std::fs::remove_dir_all(CONFIG_FILE_PATH.as_path());
        }
    }

    impl Drop for TestConfigFile {
        fn drop(&mut self) {
            Self::remove_config();
        }
    }

    #[test]
    fn empty_toml_uses_config_defaults() -> Result<(), Box<dyn Error>> {
        let config: Config = toml::from_str("")?;

        assert_eq!(config, Config::default());

        Ok(())
    }

    #[tokio::test]
    async fn read_config_creates_missing_file_with_defaults() -> Result<(), Box<dyn Error>> {
        let _config_file = TestConfigFile::new();

        let config = Config::read_config().await?;
        let written_config: Config =
            toml::from_str(&fs::read_to_string(CONFIG_FILE_PATH.as_path()).await?)?;

        assert_eq!(config, Config::default());
        assert_eq!(written_config, Config::default());

        Ok(())
    }

    #[tokio::test]
    async fn read_config_reads_existing_file() -> Result<(), Box<dyn Error>> {
        let _config_file = TestConfigFile::new();
        let expected = Config {
            multi_thread: 3,
            proxy: Some("http://127.0.0.1:8080".to_string()),
            danmu_ban_words: vec!["spoiler".to_string()],
            ..Config::default()
        };
        fs::write(
            CONFIG_FILE_PATH.as_path(),
            toml::to_string_pretty(&expected)?,
        )
        .await?;

        let config = Config::read_config().await?;

        assert_eq!(config, expected);

        Ok(())
    }

    #[tokio::test]
    async fn read_config_normalizes_out_of_range_values() -> Result<(), Box<dyn Error>> {
        let _config_file = TestConfigFile::new();
        let cases = [(0, 0, 0, 1, 1, 1), (6, 6, 7, 5, 5, 7)];

        for (
            multi_thread,
            multi_downloading_segment,
            quantity_of_logs,
            expected_multi_thread,
            expected_multi_downloading_segment,
            expected_quantity_of_logs,
        ) in cases
        {
            let config = Config {
                multi_thread,
                multi_downloading_segment,
                quantity_of_logs,
                ..Config::default()
            };
            fs::write(CONFIG_FILE_PATH.as_path(), toml::to_string_pretty(&config)?).await?;

            let normalized = Config::read_config().await?;

            assert_eq!(normalized.multi_thread, expected_multi_thread);
            assert_eq!(
                normalized.multi_downloading_segment,
                expected_multi_downloading_segment
            );
            assert_eq!(normalized.quantity_of_logs, expected_quantity_of_logs);
        }

        Ok(())
    }

    #[tokio::test]
    async fn write_config_persists_config() -> Result<(), Box<dyn Error>> {
        let _config_file = TestConfigFile::new();
        let expected = Config {
            download_resolution: DownloadResolution::P720,
            default_download_mode: DownloadMode::All,
            save_logs: false,
            ..Config::default()
        };

        expected.write_config().await?;
        let written_config: Config =
            toml::from_str(&fs::read_to_string(CONFIG_FILE_PATH.as_path()).await?)?;

        assert_eq!(written_config, expected);

        Ok(())
    }

    #[tokio::test]
    async fn read_config_reports_invalid_toml() -> Result<(), Box<dyn Error>> {
        let _config_file = TestConfigFile::new();
        fs::write(CONFIG_FILE_PATH.as_path(), "multi-thread = [").await?;

        let error = Config::read_config()
            .await
            .expect_err("invalid TOML should fail to parse");

        assert!(matches!(error, ConfigError::TomlDe(_)));

        Ok(())
    }

    #[tokio::test]
    async fn config_file_operations_report_io_errors() {
        let _config_file = TestConfigFile::new();
        std::fs::create_dir(CONFIG_FILE_PATH.as_path())
            .expect("test config directory should be created");

        let read_error = Config::read_config()
            .await
            .expect_err("reading a directory as a config file should fail");
        let write_error = Config::default()
            .write_config()
            .await
            .expect_err("writing a config to a directory should fail");

        assert!(matches!(read_error, ConfigError::IO(_)));
        assert!(matches!(write_error, ConfigError::IO(_)));
    }
}
