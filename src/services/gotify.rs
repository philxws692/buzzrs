use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::core::{NotificationFactory, NotificationService};

use anyhow::{Context, Result, anyhow};

#[derive(Deserialize, Debug)]
pub struct GotifyConfig {
    #[serde(default)]
    disable_tls: bool,
    #[serde(default)]
    priority: u8,
    #[serde(default)]
    title: String,
}

pub struct GotifyService {
    domain: String,
    token: String,
    config: GotifyConfig,
}

#[async_trait]
impl NotificationService for GotifyService {
    async fn send(&self, message: &str) -> Result<()> {
        let url = match self.config.disable_tls {
            true => format!("http://{}/message", self.domain),
            false => format!("https://{}/message", self.domain),
        };

        let body = json!({ "title": self.config.title, "message": message, "priority": self.config.priority });

        let client = reqwest::Client::new();

        let res = client
            .post(&url)
            .json(&body)
            .header("X-Gotify-Key", self.token.clone())
            .send()
            .await?;

        if res.status().is_success() {
            Ok(())
        } else {
            let status = res.status();
            let url = res.url().to_string();
            let body = res.text().await.unwrap_or_else(|_| String::new());

            Err(anyhow!(
                "sending to gotify ({}) failed with status {}: {}",
                url,
                status,
                body
            ))
        }
    }
}

pub struct GotifyFactory;

impl NotificationFactory for GotifyFactory {
    fn scheme(&self) -> &'static str {
        "gotify"
    }

    fn build(&self, url: &url::Url) -> Result<Box<dyn NotificationService>> {
        let token = url
            .path_segments()
            .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("gotify token missing in URL path"))?;

        let host = url.host_str().context("domain could not be parsed")?;
        let domain = match url.port() {
            Some(port) => format!("{}:{}", host, port),
            None => host.to_owned(),
        };

        let config: GotifyConfig = serde_urlencoded::from_str(url.query().unwrap_or(""))
            .context("could not decode url query parameters")?;
        Ok(Box::new(GotifyService {
            domain,
            token,
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

    fn gotify_url(server: &MockServer, query: &str) -> String {
        if query.is_empty() {
            format!("gotify://{}/test-token", server.address())
        } else {
            format!("gotify://{}/test-token?{}", server.address(), query)
        }
    }

    #[tokio::test]
    async fn send_posts_message_with_defaults() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/message")
                .header("x-gotify-key", "test-token")
                .json_body(json!({
                    "title": "",
                    "message": "hello",
                    "priority": 0
                }));
            then.status(200);
        });

        let service = GotifyFactory
            .build(&url::Url::parse(&gotify_url(&server, "disable_tls=true")).unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_priority_config_independently() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/message")
                .header("x-gotify-key", "test-token")
                .json_body(json!({
                    "title": "",
                    "message": "hello",
                    "priority": 7
                }));
            then.status(200);
        });

        let service = GotifyFactory
            .build(&url::Url::parse(&gotify_url(
                &server,
                "disable_tls=true&priority=7",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_title_config_independently() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/message")
                .header("x-gotify-key", "test-token")
                .json_body(json!({
                    "title": "My title",
                    "message": "hello",
                    "priority": 0
                }));
            then.status(200);
        });

        let service = GotifyFactory
            .build(&url::Url::parse(&gotify_url(
                &server,
                "disable_tls=true&title=My%20title",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_combined_config() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/message")
                .header("x-gotify-key", "test-token")
                .json_body(json!({
                    "title": "Combined",
                    "message": "hello",
                    "priority": 10
                }));
            then.status(200);
        });

        let service = GotifyFactory
            .build(&url::Url::parse(&gotify_url(
                &server,
                "disable_tls=true&title=Combined&priority=10",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_returns_error_with_official_gotify_error_payload() {
        let server = MockServer::start();
        let error_body = json!({
            "error": "Unauthorized",
            "errorCode": 401,
            "errorDescription": "you need to provide a valid access token or user credentials to access this api"
        });

        let _mock = server.mock(|when, then| {
            when.method(POST).path("/message");
            then.status(401).json_body(error_body.clone());
        });

        let service = GotifyFactory
            .build(&url::Url::parse(&gotify_url(&server, "disable_tls=true")).unwrap())
            .unwrap();

        let err = service.send("hello").await.unwrap_err().to_string();
        assert!(err.contains("failed with status 401 Unauthorized"));
        assert!(err.contains("\"error\":\"Unauthorized\""));
        assert!(err.contains("\"errorCode\":401"));
        assert!(err.contains("\"errorDescription\":"));
    }

    #[tokio::test]
    async fn send_uses_https_when_tls_not_disabled() {
        let server = MockServer::start();
        let service = GotifyFactory
            .build(&url::Url::parse(&gotify_url(&server, "")).unwrap())
            .unwrap();

        let err = service.send("hello").await.unwrap_err().to_string();
        assert!(err.contains("https://"));
        assert!(err.contains("/message"));
    }

    #[test]
    fn build_fails_when_token_missing() {
        let url = url::Url::parse("gotify://localhost").unwrap();
        let err = match GotifyFactory.build(&url) {
            Ok(_) => panic!("expected build to fail"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("gotify token missing in URL path"));
    }

    #[test]
    fn build_fails_for_invalid_query_value() {
        let url = url::Url::parse("gotify://localhost/token?priority=abc").unwrap();
        let err = match GotifyFactory.build(&url) {
            Ok(_) => panic!("expected build to fail"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("could not decode url query parameters"));
    }
}
