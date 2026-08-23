use std::{sync::Arc, time::Duration};

use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::sync::watch;
use wreq::{
    Client, IntoUrl, Proxy, RequestBuilder, Url,
    cookie::Jar,
    header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_TYPE, ORIGIN},
};
use wreq_util::Emulation;

use crate::{
    config::Config,
    constant::{self, ORIGIN_URL},
    cookie::Cookie,
    request::observable_cookie_jar::ObservableCookieJar,
};

pub(crate) mod anime_video;
pub(crate) mod common;
pub(crate) mod device_id;
pub(crate) mod observable_cookie_jar;
pub(crate) mod token;
pub(crate) mod video_src;

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("构建请求客户端失败：{0}")]
    BuildClient(#[from] wreq::Error),
    #[error("解析 url 失败：{0}")]
    UrlParseError(#[from] url::ParseError),
}

#[derive(Debug)]
pub struct RequestClient {
    cookie: Client,
    plain: Client,
}

const COOKIE_DOMAIN: &str = "gamer.com.tw";

fn add_cookie_header_to_jar(jar: &ObservableCookieJar, cookie_header: &str, url: &Url) {
    // `Jar::add_cookie_str` accepts one Set-Cookie-style record at a time, while
    // cookie.txt contains a browser Cookie request header with multiple `name=value`
    // pairs separated by semicolons. Restore their shared parent domain so the
    // cookies are available to both ani.gamer.com.tw and api.gamer.com.tw.
    for cookie in cookie_header
        .split(';')
        .map(str::trim)
        .filter(|cookie| !cookie.is_empty())
    {
        jar.add_cookie_str(&format!("{cookie}; Domain={COOKIE_DOMAIN}"), url);
    }
}

impl RequestClient {
    pub fn new(config: &Config, cookie: Cookie) -> Result<Self, RequestError> {
        let emulation = Self::get_emulation(&config.ua);

        let cookie_store = Self::get_observable_cookie_jar(cookie);

        let mut cookie_builder = Client::builder()
            .emulation(emulation)
            .timeout(Duration::from_secs(10))
            .cookie_store(true)
            .cookie_provider(cookie_store);
        let mut plain_builder = Client::builder()
            .emulation(emulation)
            .timeout(Duration::from_secs(10))
            .cookie_store(true);

        if let Some(proxy) = &config.proxy {
            cookie_builder = cookie_builder.proxy(proxy.clone());
            plain_builder = plain_builder.proxy(proxy.clone());
        }

        let cookie = cookie_builder.build()?;
        let plain = plain_builder.build()?;

        Ok(Self { cookie, plain })
    }

    pub fn get<U: IntoUrl>(&self, url: U, with_cookie: bool) -> RequestBuilder {
        let request = match with_cookie {
            true => self.cookie.get(url),
            false => self.plain.get(url),
        };

        request
            .header(
                ACCEPT_LANGUAGE,
                "zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.6",
            )
            .header(
                ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
            )
            .header(ACCEPT_ENCODING, "gzip, deflate")
            .header(CACHE_CONTROL, "max-age=0")
            .header(ORIGIN, constant::ORIGIN)
    }

    pub fn update_proxy(&self, proxy: Option<String>) -> wreq::Result<()> {
        if let Some(proxy) = proxy {
            let proxy = Proxy::all(proxy)?;
            self.cookie.update().proxies([proxy.clone()]).apply()?;
            self.plain.update().proxies([proxy]).apply()
        } else {
            self.cookie.update().unset_proxies().apply()?;
            self.plain.update().unset_proxies().apply()
        }
    }

    pub fn update_ua(&self, ua: &str) -> wreq::Result<()> {
        let emulation = Self::get_emulation(ua);
        self.cookie.update().emulation(emulation).apply()?;
        self.plain.update().emulation(emulation).apply()
    }

    pub fn update_cookies(&self, cookie: Cookie) -> wreq::Result<()> {
        let cookie_store = Self::get_observable_cookie_jar(cookie);

        self.cookie.update().cookie_provider(cookie_store).apply()?;
        self.plain
            .update()
            .cookie_provider(Arc::new(Jar::default()))
            .apply()
    }

    fn get_emulation(ua: &str) -> Emulation {
        let lowercase_ua = ua.to_ascii_lowercase();

        if lowercase_ua.contains("firefox") {
            Emulation::Firefox109
        } else if lowercase_ua.contains("edg") {
            Emulation::Edge134
        } else {
            Emulation::Chrome137
        }
    }

    fn get_observable_cookie_jar(cookie: Cookie) -> Arc<ObservableCookieJar> {
        let (tx, mut rx) = watch::channel(String::new());
        let cookie_store = Arc::new(ObservableCookieJar::new(ORIGIN_URL.clone(), tx));
        add_cookie_header_to_jar(&cookie_store, &cookie.to_string(), &ORIGIN_URL);

        tokio::spawn(async move {
            let mut cookie = cookie;
            while rx.changed().await.is_ok() {
                let cookie_string = rx.borrow_and_update().clone();
                let result = cookie.set_and_write_cookie(cookie_string).await;
                if let Err(error) = result {
                    tracing::error!(error = %error, "存储 cookie 失败");
                }
            }
        });

        cookie_store
    }
}

const MAX_LOGGED_RESPONSE_BODY: usize = 8 * 1024;

pub(crate) trait JsonResponseExt {
    async fn json_or_log<T>(self) -> wreq::Result<T>
    where
        T: DeserializeOwned;
}

impl JsonResponseExt for wreq::Response {
    async fn json_or_log<T>(self) -> wreq::Result<T>
    where
        T: DeserializeOwned,
    {
        let status = self.status();

        // avoid writing query string, such as device_id, into log
        let mut url = self.url().clone();
        url.set_query(None);

        let content_type = self
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("<unknown>")
            .to_owned();

        let body = self.bytes().await?;

        serde_json::from_slice(&body).map_err(|error| {
            let preview_len = body.len().min(MAX_LOGGED_RESPONSE_BODY);
            let body_preview = String::from_utf8_lossy(&body[..preview_len]);

            tracing::error!(%error, %status, %url, %content_type, body_len = body.len(), truncated = body.len() > preview_len, response_body = %body_preview, "JSON 响应反序列化失败");

            wreq::Error::from(error)
        })
    }
}

#[cfg(test)]
mod test {
    use wreq::{
        cookie::CookieStore,
        header::{HeaderValue, USER_AGENT},
    };

    use super::*;

    fn cookie_header(jar: &ObservableCookieJar, url: &Url) -> Option<String> {
        jar.cookies(url).map(|cookies| {
            cookies
                .to_str()
                .expect("cookie header should be valid text")
                .to_owned()
        })
    }

    fn client_cookie_header(client: &Client, url: &Url) -> Option<String> {
        client.get_cookies(url).map(|cookies| {
            cookies
                .to_str()
                .expect("cookie header should be valid text")
                .to_owned()
        })
    }

    fn config_without_system_proxy() -> Config {
        Config {
            // Avoid system proxy discovery while constructing a client in the isolated test
            // environment. No request is sent, so this address is never contacted.
            proxy: Some("http://127.0.0.1:1".to_string()),
            ..Config::default()
        }
    }

    #[test]
    fn add_cookie_header_to_jar_shares_all_pairs_across_gamer_subdomains() {
        let origin = ORIGIN_URL.clone();
        let (tx, _) = watch::channel(String::new());
        let jar = ObservableCookieJar::new(origin.clone(), tx);

        add_cookie_header_to_jar(&jar, "foo=bar; session=abc==; ; BAHAID=123", &origin);

        let expected = Some("foo=bar; session=abc==; BAHAID=123".to_owned());
        assert_eq!(cookie_header(&jar, &origin), expected);
        assert_eq!(cookie_header(&jar, &constant::API_ORIGIN_URL), expected);
    }

    #[test]
    fn add_cookie_header_to_jar_does_not_share_cookies_outside_gamer_domain() {
        let origin = ORIGIN_URL.clone();
        let unrelated = Url::parse("https://example.com").expect("test URL should be valid");
        let (tx, _) = watch::channel(String::new());
        let jar = ObservableCookieJar::new(origin.clone(), tx);

        add_cookie_header_to_jar(&jar, "session=abc", &origin);

        assert_eq!(cookie_header(&jar, &unrelated), None);
    }

    #[test]
    fn refreshed_cookie_is_persisted_and_restored_for_both_subdomains() {
        let origin = ORIGIN_URL.clone();
        let (changed, mut receiver) = watch::channel(String::new());
        let jar = ObservableCookieJar::new(origin.clone(), changed);
        add_cookie_header_to_jar(&jar, "session=old", &origin);

        let headers = [HeaderValue::from_static(
            "session=new; Domain=gamer.com.tw; Path=/",
        )];
        jar.set_cookies(&origin, &mut headers.iter());

        assert!(
            receiver
                .has_changed()
                .expect("observable jar should still own the sender")
        );
        let persisted = receiver.borrow_and_update().clone();
        assert_eq!(persisted, "session=new");

        let (restored_changed, _) = watch::channel(String::new());
        let restored = ObservableCookieJar::new(origin.clone(), restored_changed);
        add_cookie_header_to_jar(&restored, &persisted, &origin);

        let expected = Some("session=new".to_owned());
        assert_eq!(cookie_header(&restored, &origin), expected);
        assert_eq!(
            cookie_header(&restored, &constant::API_ORIGIN_URL),
            expected
        );
    }

    #[tokio::test]
    async fn get_adds_custom_headers_without_replacing_emulated_user_agent() {
        let client = RequestClient::new(&config_without_system_proxy(), Cookie::new(""))
            .expect("request client should be created");

        assert!(client.plain.headers().contains_key(USER_AGENT));

        let request = client
            .get(constant::ORIGIN, false)
            .build()
            .expect("request should be built");
        let headers = request.headers();

        assert_eq!(
            headers
                .get(ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok()),
            Some("zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.6")
        );
        assert_eq!(
            headers
                .get(ACCEPT_ENCODING)
                .and_then(|value| value.to_str().ok()),
            Some("gzip, deflate")
        );
        assert_eq!(
            headers
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("max-age=0")
        );
        assert_eq!(
            headers.get(ORIGIN).and_then(|value| value.to_str().ok()),
            Some(constant::ORIGIN)
        );
    }

    #[tokio::test]
    async fn update_cookies_replaces_the_store_used_by_authenticated_requests() {
        let client = RequestClient::new(&config_without_system_proxy(), Cookie::new("session=old"))
            .expect("request client should be created");

        assert_eq!(
            client_cookie_header(&client.cookie, &ORIGIN_URL),
            Some("session=old".to_owned())
        );

        client
            .update_cookies(Cookie::new("session=new"))
            .expect("cookie store should be replaced");

        let expected = Some("session=new".to_owned());
        assert_eq!(client_cookie_header(&client.cookie, &ORIGIN_URL), expected);
        assert_eq!(
            client_cookie_header(&client.cookie, &constant::API_ORIGIN_URL),
            expected
        );
        assert_eq!(client_cookie_header(&client.plain, &ORIGIN_URL), None);
    }

    #[tokio::test]
    async fn update_cookies_clears_authenticated_store_when_switching_to_guest() {
        let client = RequestClient::new(&config_without_system_proxy(), Cookie::default())
            .expect("request client should be created");

        client
            .update_cookies(Cookie::new("session=authenticated"))
            .expect("authenticated cookie store should be installed");
        assert_eq!(
            client_cookie_header(&client.cookie, &ORIGIN_URL),
            Some("session=authenticated".to_owned())
        );

        client
            .update_cookies(Cookie::default())
            .expect("guest cookie store should be installed");

        assert_eq!(client_cookie_header(&client.cookie, &ORIGIN_URL), None);
        assert_eq!(
            client_cookie_header(&client.cookie, &constant::API_ORIGIN_URL),
            None
        );
    }
}
