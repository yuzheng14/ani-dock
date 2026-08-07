use tokio::sync::watch;
use url::Url;
use wreq::cookie::{CookieStore, Jar};

#[derive(Debug)]
pub struct ObservableCookieJar {
    inner: Jar,
    origin: Url,
    changed: watch::Sender<String>,
}

impl ObservableCookieJar {
    fn snapshot(&self) -> String {
        self.inner
            .cookies(&self.origin)
            .and_then(|cookie| cookie.to_str().ok().map(ToOwned::to_owned))
            .unwrap_or_default()
    }

    fn notify_if_change(&self, before: String) {
        let after = self.snapshot();

        if before != after {
            self.changed.send_replace(after);
        }
    }

    pub fn new(origin: Url, changed: watch::Sender<String>) -> Self {
        Self {
            inner: Jar::default(),
            origin,
            changed,
        }
    }

    pub fn add_cookie_str(&self, cookie: &str, url: &Url) {
        self.inner.add_cookie_str(cookie, url);
    }
}

impl CookieStore for ObservableCookieJar {
    fn set_cookies(
        &self,
        url: &url::Url,
        cookie_headers: &mut dyn Iterator<Item = &wreq::header::HeaderValue>,
    ) {
        let before = self.snapshot();
        self.inner.set_cookies(url, cookie_headers);
        self.notify_if_change(before);
    }

    fn cookies(&self, url: &url::Url) -> Option<wreq::header::HeaderValue> {
        self.inner.cookies(url)
    }

    fn set_cookie(&self, url: &url::Url, cookie: &dyn wreq::cookie::IntoCookie) {
        let before = self.snapshot();
        self.inner.set_cookie(url, cookie);
        self.notify_if_change(before);
    }

    fn remove(&self, url: &url::Url, name: &str) {
        let before = self.snapshot();
        self.inner.remove(url, name);
        self.notify_if_change(before);
    }

    fn clear(&self) {
        let before = self.snapshot();
        self.inner.clear();
        self.notify_if_change(before);
    }
}

#[cfg(test)]
mod tests {
    use wreq::{cookie::Cookie, header::HeaderValue};

    use super::*;

    fn observable_jar() -> (ObservableCookieJar, watch::Receiver<String>, Url) {
        let origin = Url::parse("https://ani.gamer.com.tw/").expect("origin should be valid");
        let (changed, receiver) = watch::channel(String::new());
        let jar = ObservableCookieJar::new(origin.clone(), changed);

        (jar, receiver, origin)
    }

    fn take_change(receiver: &mut watch::Receiver<String>) -> String {
        assert!(
            receiver
                .has_changed()
                .expect("observable jar should still own the sender"),
            "cookie observer should have been notified"
        );

        receiver.borrow_and_update().clone()
    }

    fn assert_unchanged(receiver: &watch::Receiver<String>) {
        assert!(
            !receiver
                .has_changed()
                .expect("observable jar should still own the sender"),
            "cookie observer should not have been notified"
        );
    }

    #[test]
    fn add_cookie_str_seeds_jar_without_notifying_observer() {
        let (jar, receiver, origin) = observable_jar();

        jar.add_cookie_str("session=abc; Path=/", &origin);

        assert_eq!(
            jar.cookies(&origin)
                .expect("cookie should be available for its origin")
                .to_str()
                .expect("cookie header should be valid text"),
            "session=abc"
        );
        assert_unchanged(&receiver);
    }

    #[test]
    fn set_cookies_stores_cookie_and_notifies_observer() {
        let (jar, mut receiver, origin) = observable_jar();
        let headers = [HeaderValue::from_static("session=abc; Path=/")];

        jar.set_cookies(&origin, &mut headers.iter());

        assert_eq!(take_change(&mut receiver), "session=abc");
        assert_eq!(
            jar.cookies(&origin)
                .expect("cookie should be available for its origin")
                .to_str()
                .expect("cookie header should be valid text"),
            "session=abc"
        );
    }

    #[test]
    fn setting_identical_cookie_does_not_notify_observer_again() {
        let (jar, mut receiver, origin) = observable_jar();
        let headers = [HeaderValue::from_static("session=abc; Path=/")];

        jar.set_cookies(&origin, &mut headers.iter());
        assert_eq!(take_change(&mut receiver), "session=abc");

        jar.set_cookies(&origin, &mut headers.iter());

        assert_unchanged(&receiver);
    }

    #[test]
    fn set_cookie_notifies_observer() {
        let (jar, mut receiver, origin) = observable_jar();
        let cookie = Cookie::new("session", "abc");

        jar.set_cookie(&origin, &cookie);

        assert_eq!(take_change(&mut receiver), "session=abc");
    }

    #[test]
    fn remove_notifies_observer_with_current_snapshot() {
        let (jar, mut receiver, origin) = observable_jar();
        jar.set_cookie(&origin, &Cookie::new("session", "abc"));
        assert_eq!(take_change(&mut receiver), "session=abc");

        jar.remove(&origin, "session");

        assert_eq!(take_change(&mut receiver), "");
        assert!(jar.cookies(&origin).is_none());
    }

    #[test]
    fn removing_missing_cookie_does_not_notify_observer() {
        let (jar, receiver, origin) = observable_jar();

        jar.remove(&origin, "missing");

        assert_unchanged(&receiver);
    }

    #[test]
    fn clear_notifies_observer_with_empty_snapshot() {
        let (jar, mut receiver, origin) = observable_jar();
        let headers = [
            HeaderValue::from_static("session=abc; Path=/"),
            HeaderValue::from_static("device=123; Path=/"),
        ];
        jar.set_cookies(&origin, &mut headers.iter());
        assert_eq!(take_change(&mut receiver), "session=abc; device=123");

        jar.clear();

        assert_eq!(take_change(&mut receiver), "");
        assert!(jar.cookies(&origin).is_none());
    }
}
