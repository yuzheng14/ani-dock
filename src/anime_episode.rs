use std::{rc::Rc, sync::LazyLock};

use domparser::{DomNode, parse};
use indexmap::IndexMap;
use regex::Regex;
use wreq::header::REFERER;

use crate::{
    config::{Config, DownloadMode},
    constant::ORIGIN,
    cookie::Cookie,
    error::AnimeEpisodeError,
    request::RequestClient,
    sn_list::SnDetail,
};

static MIN_EPISODE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\d*\.?\d* *\.?[A-Z,a-z]*(?:電影)?\]").expect("could not parse min episode regex")
});
static FULL_EPISODE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[.+?\]").expect("could not parse full episode regex"));
static WHITESPACES_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+").expect("could not parse whitespaces regex"));

#[derive(Debug)]
pub struct AnimeEpisode {
    sn_code: String,
    download_mode: DownloadMode,
    config: Rc<Config>,
    cookie: Rc<Cookie>,
    request_client: Rc<RequestClient>,
}

// TODO client 创建与浏览器模拟
// TODO 标题集数等的正则表达式
impl AnimeEpisode {
    pub fn new(
        sn: SnDetail,
        config: Rc<Config>,
        cookie: Rc<Cookie>,
        request_client: Rc<RequestClient>,
    ) -> Self {
        Self {
            sn_code: sn.code,
            download_mode: sn
                .mode
                .unwrap_or_else(|| config.default_download_mode.clone()),
            config,
            cookie,
            request_client,
        }
    }
}

impl AnimeEpisode {
    pub async fn resolve(&self) -> Result<ResolvedAnimeEpisode, AnimeEpisodeError> {
        let url = String::from(ORIGIN) + format!("/animeVideo.php?sn={}", self.sn_code).as_str();
        let content = self
            .request_client
            .get(url.clone(), true)
            .header(REFERER, url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        self.parse_content(content)
    }

    fn parse_content(&self, content: String) -> Result<ResolvedAnimeEpisode, AnimeEpisodeError> {
        let root = parse(content);

        let title = Self::get_title(&root)?;

        let episode = Self::get_episode(&root, &title);

        let name = Self::get_name(&title, &episode);

        let episode_list = self.get_episode_list(&root, &episode)?;

        Ok(ResolvedAnimeEpisode {
            root,
            title,
            name,
            episode,
            episode_list,
        })
    }

    fn get_title(root: &DomNode) -> Result<String, AnimeEpisodeError> {
        let title = root
            .select(".anime_name  h1".to_string())
            .ok_or_else(|| {
                AnimeEpisodeError::ParseHtmlError(String::from("could not find anime's title"))
            })?
            .text();
        Ok(title)
    }

    /// Query css selector `.playing a` first.
    /// If not, then query using a strict regex.
    /// If not, then qeury using a loose regex which match last
    /// content inside `[]`.
    /// If not, then return 1.
    fn get_episode(root: &DomNode, title: &str) -> String {
        root.select(".playing a".to_string())
            .map(|dom_node| dom_node.text())
            .or_else(|| {
                MIN_EPISODE_REGEX
                    .find(title)
                    .map(|find| find.as_str()[1..find.len() - 1].into())
            })
            .or_else(|| {
                FULL_EPISODE_REGEX
                    .find_iter(title)
                    .last()
                    .map(|find| find.as_str()[1..find.len() - 1].into())
            })
            // TODO maybe a warning here?
            .unwrap_or_else(|| String::from("1"))
    }

    fn get_name(title: &str, episode: &str) -> String {
        WHITESPACES_REGEX
            .replace_all(title.replace(&format!("[{episode}]"), "").trim(), " ")
            .to_string()
    }

    fn get_episode_list(
        &self,
        root: &DomNode,
        episode: &str,
    ) -> Result<IndexMap<String, Vec<EpisodeSn>>, AnimeEpisodeError> {
        let episode_list_raw_node = {
            if !root.select_all(".season p".to_string()).is_empty() {
                let raw_p = root.select_all(".season p".to_string());
                let raw_ul = root.select_all(".season ul".to_string());

                if raw_p.len() != raw_ul.len() {
                    return Err(AnimeEpisodeError::ParseHtmlError(format!(
                        "episode category count does not match episode list count: {} != {}",
                        raw_p.len(),
                        raw_ul.len()
                    )));
                }

                raw_p
                    .into_iter()
                    .zip(raw_ul)
                    .map(|(p, ul)| (p.text(), ul.select_all("a".to_string())))
                    .collect()
            } else {
                let nodes = root.select_all(".season a".to_string());
                vec![(String::from("本篇"), nodes)]
            }
        };
        let episode_list = episode_list_raw_node
            .into_iter()
            .map(|e| {
                Ok((
                    e.0,
                    match e.1.is_empty() {
                        true => vec![EpisodeSn::new(episode, &self.sn_code)],
                        false => {
                            e.1.iter()
                                .map(|e| {
                                    Ok(EpisodeSn::new(
                                        e.text(),
                                        e.get_attribute("data-ani-video-sn".to_string())
                                            .ok_or_else(|| {
                                                AnimeEpisodeError::ParseHtmlError(format!(
                                                    "could not find sn for episode {}",
                                                    e.text()
                                                ))
                                            })?,
                                    ))
                                })
                                .collect::<Result<Vec<EpisodeSn>, AnimeEpisodeError>>()?
                        }
                    },
                ))
            })
            .collect::<Result<IndexMap<String, Vec<EpisodeSn>>, AnimeEpisodeError>>()?;
        Ok(episode_list)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeSn {
    episode: String,
    sn_code: String,
}

impl EpisodeSn {
    pub fn new(episode: impl Into<String>, sn_code: impl Into<String>) -> Self {
        Self {
            episode: episode.into(),
            sn_code: sn_code.into(),
        }
    }
}

pub struct ResolvedAnimeEpisode {
    /// 该 sn 对应的动画剧集的网页结构
    root: DomNode,
    /// 当前剧集标题
    ///
    /// example: `無職轉生～到了異世界就拿出真本事～第三季 [2]`
    title: String,
    /// 当前动画名称
    ///
    /// example: `無職轉生～到了異世界就拿出真本事～第三季`
    name: String,
    /// 当前剧集数
    ///
    /// example: 2
    episode: String,
    /// 剧集列表
    ///
    /// IndexMap<'本篇' | '特别篇' | '中文配音' | and so on, _>
    episode_list: IndexMap<String, Vec<EpisodeSn>>,
}

#[cfg(test)]
mod test {
    use std::{error::Error, path::PathBuf};

    use tokio::fs;

    use super::*;

    fn new_test_anime_episode(sn_code: &str) -> AnimeEpisode {
        let config = Rc::new(Config {
            // 显式代理会关闭 wreq 的系统代理自动发现，避免隔离测试环境中读取
            // macOS SystemConfiguration 失败并毒化其全局 LazyLock。这里的解析测试
            // 不会发起网络请求，因此不会实际连接这个占位地址。
            proxy: Some("http://127.0.0.1:1".to_string()),
            ..Config::default()
        });
        let cookie = Rc::new(Cookie::new(""));
        let request_client = Rc::new(
            RequestClient::new(&config, &cookie).expect("test request client should be created"),
        );

        AnimeEpisode::new(
            SnDetail {
                code: sn_code.to_string(),
                mode: None,
            },
            config,
            cookie,
            request_client,
        )
    }

    #[tokio::test]
    async fn parse_anime_episode() -> Result<(), Box<dyn Error>> {
        let content = fs::read_to_string(
            PathBuf::from(".")
                .join("tests")
                .join("fixture")
                .join("animeVideo-sn-49903.html"),
        )
        .await?;

        let anime_episode = new_test_anime_episode("49903");
        let resolved_anime_episode = anime_episode.parse_content(content)?;

        assert_eq!(
            resolved_anime_episode.title,
            String::from("無職轉生～到了異世界就拿出真本事～第三季 [2]")
        );
        assert_eq!(resolved_anime_episode.episode, String::from("2"));
        assert_eq!(
            resolved_anime_episode.name,
            String::from("無職轉生～到了異世界就拿出真本事～第三季")
        );
        assert_eq!(
            resolved_anime_episode.episode_list,
            IndexMap::from([(
                String::from("本篇"),
                vec![EpisodeSn::new("1", "49902"), EpisodeSn::new("2", "49903")]
            )])
        );

        Ok(())
    }

    #[tokio::test]
    async fn parse_anime_episode_without_episode_groups() -> Result<(), Box<dyn Error>> {
        let content = fs::read_to_string(
            PathBuf::from(".")
                .join("tests")
                .join("fixture")
                .join("animeVideo-sn-49780.html"),
        )
        .await?;

        let anime_episode = new_test_anime_episode("49780");
        let resolved_anime_episode = anime_episode.parse_content(content)?;

        assert_eq!(
            resolved_anime_episode.title,
            "劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇 [電影]"
        );
        assert_eq!(resolved_anime_episode.episode, "電影");
        assert_eq!(
            resolved_anime_episode.name,
            "劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇"
        );
        assert_eq!(
            resolved_anime_episode.episode_list,
            IndexMap::from([("本篇".to_string(), vec![EpisodeSn::new("電影", "49780")],)])
        );

        Ok(())
    }

    #[tokio::test]
    async fn parse_anime_episode_with_multi_season() -> Result<(), Box<dyn Error>> {
        let content = fs::read_to_string(
            PathBuf::from(".")
                .join("tests")
                .join("fixture")
                .join("animeVideo-sn-3499.html"),
        )
        .await?;

        let anime_episode = new_test_anime_episode("3499");
        let resolved_anime_episode = anime_episode.parse_content(content)?;

        assert_eq!(resolved_anime_episode.title, "進擊的巨人 [1]");
        assert_eq!(resolved_anime_episode.name, "進擊的巨人");
        assert_eq!(resolved_anime_episode.episode, "1");
        assert_eq!(
            resolved_anime_episode
                .episode_list
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["本篇", "中文配音"]
        );

        let main_episodes = resolved_anime_episode
            .episode_list
            .get("本篇")
            .expect("main episodes should exist");
        assert_eq!(main_episodes.len(), 25);
        assert_eq!(main_episodes.first(), Some(&EpisodeSn::new("1", "3499")));
        assert_eq!(main_episodes[2], EpisodeSn::new("3", "3514"));
        assert_eq!(main_episodes.last(), Some(&EpisodeSn::new("25", "3513")));

        let dubbed_episodes = resolved_anime_episode
            .episode_list
            .get("中文配音")
            .expect("dubbed episodes should exist");
        assert_eq!(dubbed_episodes.len(), 25);
        assert_eq!(dubbed_episodes.first(), Some(&EpisodeSn::new("1", "20273")));
        assert_eq!(dubbed_episodes.last(), Some(&EpisodeSn::new("25", "20297")));

        Ok(())
    }

    #[test]
    fn get_title_returns_anime_title() {
        let content = r#"
            <h1>頁面中的其他標題</h1>
            <div class="anime_name">
                <h1>測試動畫 [1]</h1>
            </div>
        "#;
        let root = parse(content.to_string());

        let title = AnimeEpisode::get_title(&root).expect("anime title should exist");

        assert_eq!(title, "測試動畫 [1]");
    }

    #[test]
    fn get_title_reports_missing_title() {
        let content = r#"
            <section class="season">
                <ul>
                    <li class="playing">
                        <a data-ani-video-sn="1">1</a>
                    </li>
                </ul>
            </section>
        "#;
        let root = parse(content.to_string());

        let result = AnimeEpisode::get_title(&root);

        assert!(matches!(
            result,
            Err(AnimeEpisodeError::ParseHtmlError(message))
                if message == "could not find anime's title"
        ));
    }

    #[test]
    fn get_episode_prefers_playing_episode_over_title() {
        let content = r#"
            <section class="season">
                <ul>
                    <li class="playing"><a>7</a></li>
                </ul>
            </sertion>
        "#;
        let root = parse(content.to_string());

        let episode = AnimeEpisode::get_episode(&root, "測試動畫 [99]");

        assert_eq!(episode, "7");
    }

    #[test]
    fn get_episode_extracts_strict_title_formats() {
        let root = parse(String::new());

        for (title, expected) in [
            ("測試動畫 [1]", "1"),
            ("測試動畫 [1.5]", "1.5"),
            ("測試動畫 [12 SP]", "12 SP"),
            ("測試動畫 [SP]", "SP"),
            ("測試動畫 [OVA]", "OVA"),
            ("測試動畫 [電影]", "電影"),
        ] {
            assert_eq!(
                AnimeEpisode::get_episode(&root, title),
                expected,
                "failed to extract episode from {title}"
            );
        }
    }

    #[test]
    fn get_episode_prefers_strict_match_over_later_loose_match() {
        let root = parse(String::new());

        let episode = AnimeEpisode::get_episode(&root, "測試動畫 [12] [中文配音]");

        assert_eq!(episode, "12");
    }

    #[test]
    fn get_episode_uses_last_loose_title_match() {
        let root = parse(String::new());

        let episode = AnimeEpisode::get_episode(&root, "測試動畫 [中文配音] [特別篇]");

        assert_eq!(episode, "特別篇");
    }

    #[test]
    fn get_episode_defaults_to_one() {
        let root = parse(String::new());

        let episode = AnimeEpisode::get_episode(&root, "沒有集數標記的動畫");

        assert_eq!(episode, "1");
    }

    #[test]
    fn get_name_removes_episode_and_normalizes_whitespace() {
        for (title, episode, expected) in [
            ("測試動畫 [1]", "1", "測試動畫"),
            ("  測試   動畫\n第三季 [1.5]  ", "1.5", "測試 動畫 第三季"),
            ("測試動畫 [中文配音] [12]", "12", "測試動畫 [中文配音]"),
            ("沒有集數標記的動畫", "1", "沒有集數標記的動畫"),
        ] {
            assert_eq!(
                AnimeEpisode::get_name(title, episode),
                expected,
                "failed to extract anime name from {title}"
            );
        }
    }

    #[test]
    fn get_episode_list_uses_default_main_category() {
        let content = r#"
            <section class="season">
                <ul>
                    <li><a data-ani-video-sn="101">1</a></li>
                    <li><a data-ani-video-sn="102">2</a></li>
                </ul>
            </section>
        "#;
        let root = parse(content.to_string());
        let anime_episode = new_test_anime_episode("101");

        let episode_list = anime_episode
            .get_episode_list(&root, "1")
            .expect("episode list should parse");

        assert_eq!(
            episode_list,
            IndexMap::from([(
                "本篇".to_string(),
                vec![EpisodeSn::new("1", "101"), EpisodeSn::new("2", "102")],
            )])
        );
    }

    #[test]
    fn get_episode_list_preserves_categories_and_episode_order() {
        let content = r#"
            <section class="season">
                <p>本篇</p>
                <ul>
                    <li><a data-ani-video-sn="101">1</a></li>
                    <li><a data-ani-video-sn="102">2</a></li>
                </ul>
                <p>中文配音</p>
                <ul>
                    <li><a data-ani-video-sn="201">1</a></li>
                    <li><a data-ani-video-sn="202">2</a></li>
                </ul>
            </section>
        "#;
        let root = parse(content.to_string());
        let anime_episode = new_test_anime_episode("101");

        let episode_list = anime_episode
            .get_episode_list(&root, "1")
            .expect("episode list should parse");

        assert_eq!(
            episode_list,
            IndexMap::from([
                (
                    "本篇".to_string(),
                    vec![EpisodeSn::new("1", "101"), EpisodeSn::new("2", "102")],
                ),
                (
                    "中文配音".to_string(),
                    vec![EpisodeSn::new("1", "201"), EpisodeSn::new("2", "202")],
                ),
            ])
        );
    }

    #[test]
    fn get_episode_list_reports_missing_episode_sn() {
        let content = r#"
            <section class="season">
                <ul>
                    <li><a>1</a></li>
                </ul>
            </section>
        "#;
        let root = parse(content.to_string());
        let anime_episode = new_test_anime_episode("101");

        let result = anime_episode.get_episode_list(&root, "1");

        assert!(matches!(
            result,
            Err(AnimeEpisodeError::ParseHtmlError(message))
                if message == "could not find sn for episode 1"
        ));
    }

    #[test]
    fn get_episode_list_reports_mismatched_episode_groups() {
        let content = r#"
            <section class="season">
                <p>本篇</p>
                <ul>
                    <li><a data-ani-video-sn="1">1</a></li>
                </ul>
                <p>中文配音</p>
            </section>
        "#;
        let root = parse(content.to_string());
        let anime_episode = new_test_anime_episode("101");

        let result = anime_episode.get_episode_list(&root, "1");

        assert!(matches!(
            result,
            Err(AnimeEpisodeError::ParseHtmlError(message))
                if message
                    == "episode category count does not match episode list count: 2 != 1"
        ));
    }

    #[test]
    fn get_episode_list_uses_current_sn_when_season_is_empty() {
        let root = parse(String::new());
        let anime_episode = new_test_anime_episode("777");

        let episode_list = anime_episode
            .get_episode_list(&root, "電影")
            .expect("single episode list should parse");

        assert_eq!(
            episode_list,
            IndexMap::from([("本篇".to_string(), vec![EpisodeSn::new("電影", "777")],)])
        );
    }
}
