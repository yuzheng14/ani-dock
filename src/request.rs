use std::{sync::Arc, time::Duration};

use wreq::{
    Client, IntoUrl, RequestBuilder, Url, cookie,
    header::{ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CACHE_CONTROL, ORIGIN},
};
use wreq_util::Emulation;

use crate::{
    config::Config,
    constant::{self},
    cookie::Cookie,
    error::RequestError,
};

#[derive(Debug)]
pub struct RequestClient {
    cookie: Client,
    plain: Client,
}

fn add_cookie_header_to_jar(jar: &cookie::Jar, cookie_header: &str, url: &Url) {
    // `Jar::add_cookie_str` accepts one Set-Cookie-style record at a time, while
    // cookie.txt contains a browser Cookie request header with multiple `name=value`
    // pairs separated by semicolons.
    for cookie in cookie_header
        .split(';')
        .map(str::trim)
        .filter(|cookie| !cookie.is_empty())
    {
        jar.add_cookie_str(cookie, url);
    }
}

impl RequestClient {
    pub fn new(config: &Config, cookie: &Cookie) -> Result<Self, RequestError> {
        let lowercase_ua = config.ua.to_ascii_lowercase();
        let emulation = if lowercase_ua.contains("firefox") {
            Emulation::Firefox109
        } else if lowercase_ua.contains("edg") {
            Emulation::Edge134
        } else {
            Emulation::Chrome137
        };

        let cookie_store = Arc::new(cookie::Jar::default());
        add_cookie_header_to_jar(
            &cookie_store,
            cookie.as_str(),
            &constant::ORIGIN.parse::<Url>()?,
        );

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
}

#[cfg(test)]
mod test {
    use wreq::{cookie::CookieStore, header::USER_AGENT};

    use super::*;

    #[test]
    fn add_cookie_header_to_jar_preserves_all_cookie_pairs() {
        let url = constant::ORIGIN
            .parse::<Url>()
            .expect("origin should be a valid URL");
        let jar = cookie::Jar::default();

        add_cookie_header_to_jar(&jar, "foo=bar; session=abc==; ; BAHAID=123", &url);

        let cookies = jar
            .cookies(&url)
            .expect("cookies should have been added to the jar");

        assert_eq!(
            cookies
                .to_str()
                .expect("cookie header should be valid text"),
            "foo=bar; session=abc==; BAHAID=123"
        );
    }

    #[test]
    fn get_adds_custom_headers_without_replacing_emulated_user_agent() {
        let config = Config {
            // Avoid system proxy discovery while constructing a client in the isolated test
            // environment. No request is sent, so this address is never contacted.
            proxy: Some("http://127.0.0.1:1".to_string()),
            ..Config::default()
        };
        let client = RequestClient::new(&config, &Cookie::new(""))
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
}
