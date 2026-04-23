use async_trait::async_trait;
use serde::{Deserialize, Deserializer};

use anyhow::{Context, Result, anyhow};
use serde_json::json;

use crate::core::{NotificationFactory, NotificationService};

#[derive(Deserialize, Debug)]
/// Query-backed configuration for `ntfy://` URLs.
///
/// Docs:
/// - Sending messages: <https://docs.ntfy.sh/publish/>
pub struct NtfyConfig {
    /// Attachment URL.
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#attach-file-from-a-url>
    #[serde(default)]
    attach: String,
    /// Toggle server-side message caching.
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#message-caching>
    #[serde(default = "default_cache")]
    cache: bool,
    /// URL opened when notification is clicked.
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#click-action>
    #[serde(default)]
    click: String,
    /// Scheduled delivery value (`delay`, `in`, or `at`).
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#scheduled-delivery>
    #[serde(default, alias = "in", alias = "at")]
    delay: String,
    /// E-mail recipient for notification forwarding.
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#e-mail-notifications>
    #[serde(default)]
    email: String,
    /// Notification icon URL.
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#icons>
    #[serde(default)]
    icon: String,
    /// Attachment filename override.
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#attachments>
    #[serde(default)]
    filename: String,
    /// Notification priority (`1..=5`).
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#message-priority>
    #[serde(default)]
    priority: u8,
    /// Use plain HTTP instead of HTTPS (internal transport toggle).
    #[serde(default)]
    disable_tls: bool,
    /// Notification tags.
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#tags-emojis>
    #[serde(default, deserialize_with = "deserialize_tags")]
    tags: Vec<String>,
    /// Notification title.
    ///
    /// Docs: <https://docs.ntfy.sh/publish/#message-title>
    #[serde(default)]
    title: String,
}

/// Sends notifications to ntfy publish endpoint.
pub struct NtfyService {
    domain: String,
    token: String,
    topic: String,
    config: NtfyConfig,
}

fn default_cache() -> bool {
    true
}

/// Supports `tags=a,b` in URL query and normalizes to `Vec<String>`.
fn deserialize_tags<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Tags {
        List(Vec<String>),
        Single(String),
    }

    let maybe_tags = Option::<Tags>::deserialize(deserializer)?;
    Ok(match maybe_tags {
        Some(Tags::List(tags)) => tags,
        Some(Tags::Single(tags)) => tags
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_owned)
            .collect(),
        None => Vec::new(),
    })
}

#[async_trait]
impl NotificationService for NtfyService {
    async fn send(&self, message: &str) -> Result<()> {
        let url = match self.config.disable_tls {
            true => format!("http://{}", self.domain),
            false => format!("https://{}", self.domain),
        };

        let client = reqwest::Client::new();

        let body = json!({
            "topic": self.topic,
            "message": message,
            "title": self.config.title,
            "tags": self.config.tags,
            "priority": self.config.priority,
            "click": self.config.click,
            "attach": self.config.attach,
            "icon": self.config.icon,
            "filename": self.config.filename,
            "delay": self.config.delay,
            "email": self.config.email,
        });

        let cache = match self.config.cache {
            true => "yes",
            false => "no",
        };

        let res = client
            .post(&url)
            .json(&body)
            .bearer_auth(self.token.clone())
            .header("X-Cache", cache)
            .send()
            .await?;

        if res.status().is_success() {
            Ok(())
        } else {
            let status = res.status();
            let url = res.url().to_string();
            let body = res.text().await.unwrap_or_else(|_| String::new());

            Err(anyhow!(
                "sending to ntfy ({}) failed with status {}: {}",
                url,
                status,
                body
            ))
        }
    }
}

pub struct NtfyFactory;

impl NotificationFactory for NtfyFactory {
    fn scheme(&self) -> &'static str {
        "ntfy"
    }

    fn build(&self, url: &url::Url) -> Result<Box<dyn NotificationService>> {
        // URL format: ntfy://<host>[:port]/<token>/<topic>?title=...&priority=...
        let segments: Vec<_> = url
            .path_segments()
            .map(|segments| segments.filter(|segment| !segment.is_empty()).collect())
            .unwrap_or_default();

        let token = segments
            .first()
            .copied()
            .ok_or_else(|| anyhow!("ntfy token missing in URL path"))?
            .to_owned();

        let topic = segments
            .get(1)
            .copied()
            .ok_or_else(|| anyhow!("ntfy topic missing in URL path"))?
            .to_owned();

        let host = url.host_str().context("domain could not be parsed")?;
        let domain = match url.port() {
            Some(port) => format!("{}:{}", host, port),
            None => host.to_owned(),
        };

        let config: NtfyConfig = serde_urlencoded::from_str(url.query().unwrap_or(""))
            .context("could not decode url query parameters")?;

        Ok(Box::new(NtfyService {
            domain,
            token,
            topic,
            config,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use serde_json::json;

    fn ntfy_url(server: &MockServer, query: &str) -> String {
        if query.is_empty() {
            format!("ntfy://{}/test-token/test-topic", server.address())
        } else {
            format!("ntfy://{}/test-token/test-topic?{}", server.address(), query)
        }
    }

    fn expected_default_body() -> serde_json::Value {
        json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "",
            "email": ""
        })
    }

    #[tokio::test]
    async fn send_posts_message_with_defaults() {
        let server = MockServer::start();
        let body = expected_default_body();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/")
                .header("authorization", "Bearer test-token")
                .header("x-cache", "yes")
                .json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "disable_tls=true")).unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_cache_config_independently() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/")
                .header("x-cache", "no")
                .json_body(expected_default_body());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "disable_tls=true&cache=false")).unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_priority_config_independently() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 5,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "",
            "email": ""
        });

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/")
                .header("x-cache", "yes")
                .json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "disable_tls=true&priority=5")).unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_title_config_independently() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "Title",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "",
            "email": ""
        });

        let mock = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(
                &server,
                "disable_tls=true&title=Title",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_tags_config_independently() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": ["ops", "alert"],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "",
            "email": ""
        });

        let mock = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(
                &server,
                "disable_tls=true&tags=ops,alert",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_click_config_independently() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "https://example.com",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "",
            "email": ""
        });

        let mock = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(
                &server,
                "disable_tls=true&click=https%3A%2F%2Fexample.com",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_attach_config_independently() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "https://example.com/file.txt",
            "icon": "",
            "filename": "",
            "delay": "",
            "email": ""
        });

        let mock = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(
                &server,
                "disable_tls=true&attach=https%3A%2F%2Fexample.com%2Ffile.txt",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_icon_config_independently() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "https://example.com/icon.png",
            "filename": "",
            "delay": "",
            "email": ""
        });

        let mock = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(
                &server,
                "disable_tls=true&icon=https%3A%2F%2Fexample.com%2Ficon.png",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_filename_config_independently() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "report.txt",
            "delay": "",
            "email": ""
        });

        let mock = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(
                &server,
                "disable_tls=true&filename=report.txt",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_delay_config_and_aliases_independently() {
        let server = MockServer::start();
        let body_delay = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "30m",
            "email": ""
        });

        let body_in = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "2h",
            "email": ""
        });

        let body_at = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "tomorrow",
            "email": ""
        });

        let mock_delay = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body_delay.clone());
            then.status(200);
        });
        let mock_in = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body_in.clone());
            then.status(200);
        });
        let mock_at = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body_at.clone());
            then.status(200);
        });

        let service_delay = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "disable_tls=true&delay=30m")).unwrap())
            .unwrap();
        service_delay.send("hello").await.unwrap();

        let service_in = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "disable_tls=true&in=2h")).unwrap())
            .unwrap();
        service_in.send("hello").await.unwrap();

        let service_at = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "disable_tls=true&at=tomorrow")).unwrap())
            .unwrap();
        service_at.send("hello").await.unwrap();

        mock_delay.assert();
        mock_in.assert();
        mock_at.assert();
    }

    #[tokio::test]
    async fn send_applies_email_config_independently() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "",
            "tags": [],
            "priority": 0,
            "click": "",
            "attach": "",
            "icon": "",
            "filename": "",
            "delay": "",
            "email": "ops@example.com"
        });

        let mock = server.mock(|when, then| {
            when.method(POST).path("/").json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(
                &server,
                "disable_tls=true&email=ops%40example.com",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_combined_config() {
        let server = MockServer::start();
        let body = json!({
            "topic": "test-topic",
            "message": "hello",
            "title": "Combined",
            "tags": ["warning", "backup"],
            "priority": 4,
            "click": "https://example.com/dashboard",
            "attach": "https://example.com/file.bin",
            "icon": "https://example.com/icon.png",
            "filename": "snapshot.bin",
            "delay": "30m",
            "email": "alerts@example.com"
        });

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/")
                .header("authorization", "Bearer test-token")
                .header("x-cache", "no")
                .json_body(body.clone());
            then.status(200);
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(
                &server,
                "disable_tls=true&cache=false&title=Combined&tags=warning,backup&priority=4&click=https%3A%2F%2Fexample.com%2Fdashboard&attach=https%3A%2F%2Fexample.com%2Ffile.bin&icon=https%3A%2F%2Fexample.com%2Ficon.png&filename=snapshot.bin&delay=30m&email=alerts%40example.com",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_returns_error_with_official_ntfy_unauthorized_payload() {
        let server = MockServer::start();
        let error_body = json!({
            "code": 40101,
            "http": 401,
            "error": "unauthorized",
            "link": "https://ntfy.sh/docs/publish/#authentication"
        });

        let _mock = server.mock(|when, then| {
            when.method(POST).path("/");
            then.status(401).json_body(error_body.clone());
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "disable_tls=true")).unwrap())
            .unwrap();

        let err = service.send("hello").await.unwrap_err().to_string();
        assert!(err.contains("failed with status 401 Unauthorized"));
        assert!(err.contains("\"code\":40101"));
        assert!(err.contains("\"http\":401"));
        assert!(err.contains("\"error\":\"unauthorized\""));
    }

    #[tokio::test]
    async fn send_returns_error_with_official_ntfy_rate_limit_payload() {
        let server = MockServer::start();
        let error_body = json!({
            "code": 42908,
            "http": 429,
            "error": "limit reached: daily message quota reached",
            "link": "https://ntfy.sh/docs/publish/#limitations"
        });

        let _mock = server.mock(|when, then| {
            when.method(POST).path("/");
            then.status(429).json_body(error_body.clone());
        });

        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "disable_tls=true")).unwrap())
            .unwrap();

        let err = service.send("hello").await.unwrap_err().to_string();
        assert!(err.contains("failed with status 429 Too Many Requests"));
        assert!(err.contains("\"code\":42908"));
        assert!(err.contains("\"http\":429"));
        assert!(err.contains("daily message quota reached"));
    }

    #[tokio::test]
    async fn send_uses_https_when_tls_not_disabled() {
        let server = MockServer::start();
        let service = NtfyFactory
            .build(&url::Url::parse(&ntfy_url(&server, "")).unwrap())
            .unwrap();

        let err = service.send("hello").await.unwrap_err().to_string();
        assert!(err.contains("https://"));
    }

    #[test]
    fn build_fails_when_token_missing() {
        let url = url::Url::parse("ntfy://localhost").unwrap();
        let err = match NtfyFactory.build(&url) {
            Ok(_) => panic!("expected build to fail"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("ntfy token missing in URL path"));
    }

    #[test]
    fn build_fails_when_topic_missing() {
        let url = url::Url::parse("ntfy://localhost/token").unwrap();
        let err = match NtfyFactory.build(&url) {
            Ok(_) => panic!("expected build to fail"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("ntfy topic missing in URL path"));
    }

    #[test]
    fn build_fails_for_invalid_query_value() {
        let url = url::Url::parse("ntfy://localhost/token/topic?cache=maybe").unwrap();
        let err = match NtfyFactory.build(&url) {
            Ok(_) => panic!("expected build to fail"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("could not decode url query parameters"));
    }
}
