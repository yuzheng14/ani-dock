use ani_dock_core::{Config, Cookie, DownloadResolution, InternalConfig};
use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{ApiResult, router::AppState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsConfig {
    pub download_resolution: DownloadResolution,
    pub lock_resolution: bool,
    pub only_use_vip: bool,
    pub multi_downloading_segment: usize,
    pub ua: String,
    pub proxy: Option<String>,
    pub ads_time: u32,
    pub internal: InternalConfig,
}

impl From<SettingsConfig> for Config {
    fn from(value: SettingsConfig) -> Self {
        Self {
            download_resolution: value.download_resolution,
            lock_resolution: value.lock_resolution,
            only_use_vip: value.only_use_vip,
            multi_downloading_segment: value.multi_downloading_segment,
            ua: value.ua,
            proxy: value.proxy,
            ads_time: value.ads_time,
            internal: value.internal,
        }
    }
}

impl From<Config> for SettingsConfig {
    fn from(value: Config) -> Self {
        Self {
            download_resolution: value.download_resolution,
            lock_resolution: value.lock_resolution,
            only_use_vip: value.only_use_vip,
            multi_downloading_segment: value.multi_downloading_segment,
            ua: value.ua,
            proxy: value.proxy,
            ads_time: value.ads_time,
            internal: value.internal,
        }
    }
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Config::default().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    #[serde(flatten)]
    config: SettingsConfig,
    cookie: Cookie,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/", get(get_settings).put(update_settings))
}

async fn get_settings(State(state): State<AppState>) -> Json<Settings> {
    let settings = Settings {
        config: state.config.lock().unwrap().clone().into(),
        cookie: state.cookie,
    };

    Json(settings)
}

async fn update_settings(
    State(mut state): State<AppState>,
    Json(settings): Json<Settings>,
) -> ApiResult {
    let config: Config = settings.config.into();
    config.write_config().await?;
    *state.config.lock().unwrap() = config.clone();

    // FIXME state is cloned, so modify cookie here will not influence global state
    state
        .cookie
        .set_and_write_cookie(settings.cookie.0.clone())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use ani_dock_core::{Config, ConfigVersion, Cookie, DownloadResolution, InternalConfig};
    use serde_json::json;

    use crate::router::settings::SettingsConfig;

    use super::Settings;

    fn fixture_settings() -> Settings {
        Settings {
            config: SettingsConfig {
                lock_resolution: true,
                only_use_vip: false,
                multi_downloading_segment: 3,
                ua: "Mozilla/5.0 (test)".to_owned(),
                proxy: Some("http://127.0.0.1:7890".to_owned()),
                ads_time: 30,
                ..SettingsConfig::default()
            },
            cookie: Cookie::new("foo=bar"),
        }
    }

    #[test]
    fn settings_serializes_config_fields_flat() -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(fixture_settings())?;

        assert!(
            value.get("config").is_none(),
            "config 字段应平铺展开，不应嵌套在 config 键下"
        );
        assert_eq!(value["downloadResolution"], json!("1080"));
        assert_eq!(value["lockResolution"], json!(true));
        assert_eq!(value["onlyUseVip"], json!(false));
        assert_eq!(value["multiDownloadingSegment"], json!(3));
        assert_eq!(value["ua"], json!("Mozilla/5.0 (test)"));
        assert_eq!(value["proxy"], json!("http://127.0.0.1:7890"));
        assert_eq!(value["adsTime"], json!(30));
        assert_eq!(value["cookie"], json!("foo=bar"));
        Ok(())
    }

    #[test]
    fn settings_round_trips_through_json() -> Result<(), serde_json::Error> {
        let original = fixture_settings();
        let json = serde_json::to_string(&original)?;
        let restored: Settings = serde_json::from_str(&json)?;

        assert_eq!(restored, original);
        Ok(())
    }

    #[test]
    fn settings_deserializes_partial_flat_config_with_defaults() -> Result<(), serde_json::Error> {
        let settings: Settings = serde_json::from_value(json!({
            "lockResolution": true,
            "cookie": "a=b",
        }))?;

        assert_eq!(settings.cookie, Cookie::new("a=b"));
        assert!(settings.config.lock_resolution);
        assert_eq!(
            settings.config.download_resolution,
            Config::default().download_resolution
        );
        assert_eq!(settings.config.ua, Config::default().ua);
        assert_eq!(settings.config.ads_time, Config::default().ads_time);
        Ok(())
    }

    #[test]
    fn settings_config_converts_to_config_and_back_preserving_all_fields() {
        let config = Config {
            download_resolution: DownloadResolution::P720,
            lock_resolution: true,
            only_use_vip: true,
            multi_downloading_segment: 4,
            ua: "Mozilla/5.0 (custom)".to_owned(),
            proxy: Some("http://127.0.0.1:8080".to_owned()),
            ads_time: 40,
            internal: InternalConfig {
                config_version: ConfigVersion { major: 2, minor: 1 },
            },
        };

        let settings_config = SettingsConfig::from(config.clone());
        assert_eq!(Config::from(settings_config.clone()), config);
        assert_eq!(SettingsConfig::from(config), settings_config);
    }

    #[test]
    fn settings_config_default_matches_config_default() {
        assert_eq!(Config::from(SettingsConfig::default()), Config::default());
    }

    #[test]
    fn settings_round_trip_preserves_internal_config_version() {
        let settings = Settings {
            config: SettingsConfig {
                internal: InternalConfig {
                    config_version: ConfigVersion { major: 2, minor: 5 },
                },
                ..SettingsConfig::default()
            },
            cookie: Cookie::new("a=b"),
        };

        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, settings);
        assert_eq!(
            restored.config.internal.config_version,
            ConfigVersion { major: 2, minor: 5 }
        );
    }

    #[test]
    fn settings_serializes_internal_with_snake_case_config_version() {
        let settings = fixture_settings();
        let value = serde_json::to_value(settings).unwrap();

        assert_eq!(value["internal"]["config_version"]["major"], json!(1));
        assert_eq!(value["internal"]["config_version"]["minor"], json!(0));
    }
}
