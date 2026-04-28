use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::core::{NotificationFactory, NotificationService};

#[derive(Deserialize, Debug, Default, PartialEq, Eq)]
/// Query-backed configuration for `discord://` URLs.
///
/// Docs:
/// - Execute webhook: <https://discord.com/developers/docs/resources/webhook#execute-webhook>
pub struct DiscordConfig {
    /// Webhook avatar URL override.
    ///
    /// Docs: <https://discord.com/developers/docs/resources/webhook#execute-webhook>
    #[serde(default = "default_avatar", alias = "avatarurl")]
    avatar: String,
    /// Embed color as hex string (e.g. `FF9900` or `0xFF9900`).
    ///
    /// Docs: <https://discord.com/developers/docs/resources/channel#embed-object-embed-structure>
    #[serde(default = "default_color")]
    color: String,
    /// Embed title.
    ///
    /// Docs: <https://discord.com/developers/docs/resources/channel#embed-object-embed-structure>
    #[serde(default = "default_title")]
    title: String,
    /// Webhook username override.
    ///
    /// Docs: <https://discord.com/developers/docs/resources/webhook#execute-webhook>
    #[serde(default = "default_username")]
    username: String,
    /// Base URL for webhook requests (primarily for testing).
    ///
    /// Docs: <https://discord.com/developers/docs/resources/webhook#execute-webhook>
    #[serde(default = "default_base_url", alias = "baseurl")]
    base_url: String,
}

/// Sends notifications to Discord webhooks.
pub struct DiscordService {
    token: String,
    webhook_id: String,
    config: DiscordConfig,
}

fn default_username() -> String {
    "Buzz".to_owned()
}

fn default_base_url() -> String {
    "https://discord.com".to_owned()
}

fn default_title() -> String {
    "New notification from Buzz".to_owned()
}

fn default_avatar() -> String {
    "https://raw.githubusercontent.com/philxws692/buzzrs/refs/heads/main/assets/default_avatar.png"
        .to_owned()
}

fn default_color() -> String {
    "FF9900".to_owned()
}

fn uses_default_embed_config(config: &DiscordConfig) -> bool {
    config.username == default_username()
        && config.title == default_title()
        && config.avatar == default_avatar()
        && config.color == default_color()
}

#[async_trait]
impl NotificationService for DiscordService {
    async fn send(&self, message: &str) -> Result<()> {
        let base_url = self.config.base_url.trim_end_matches('/');
        let url = format!("{}/api/webhooks/{}/{}", base_url, self.webhook_id, self.token);

        let client = reqwest::Client::new();

        let color_value = self
            .config
            .color
            .strip_prefix("0x")
            .or_else(|| self.config.color.strip_prefix("0X"))
            .unwrap_or(&self.config.color);

        let embeds = if uses_default_embed_config(&self.config) {
            Vec::new()
        } else {
            vec![json!({
                "title": self.config.title,
                "description": message,
                "color": u32::from_str_radix(color_value, 16)?
            })]
        };

        let body = json!({
            "username": self.config.username,
            "avatar_url": self.config.avatar,
            "embeds": embeds
        });

        let res = client.post(&url).json(&body).send().await?;

        if res.status().is_success() {
            Ok(())
        } else {
            let status = res.status();
            let url = res.url().to_string();
            let body = res.text().await.unwrap_or_else(|_| String::new());

            Err(anyhow!(
                "sending to discord ({}) failed with status {}: {}",
                url,
                status,
                body
            ))
        }
    }
}

pub struct DiscordFactory;

impl NotificationFactory for DiscordFactory {
    fn scheme(&self) -> &'static str {
        "discord"
    }

    fn build(&self, url: &url::Url) -> Result<Box<dyn NotificationService>> {
        // URL format: discord://<token>@<webhook_id>?username=...&avatar=...
        let webhook_id = url
            .host_str()
            .filter(|host| !host.is_empty())
            .ok_or_else(|| anyhow!("webhook id missing in discord URL"))?
            .to_owned();

        let token = url.username().to_owned();
        if token.is_empty() {
            return Err(anyhow!("webhook token missing in discord URL"));
        }

        let config: DiscordConfig = serde_urlencoded::from_str(url.query().unwrap_or(""))
            .context("could not decode url query parameters")?;

        Ok(Box::new(DiscordService {
            token,
            webhook_id,
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

    fn discord_url(server: &MockServer, query: &str) -> String {
        if query.is_empty() {
            format!(
                "discord://test-token@webhook-id?base_url={}",
                urlencoding::encode(&server.base_url())
            )
        } else {
            format!(
                "discord://test-token@webhook-id?base_url={}&{}",
                urlencoding::encode(&server.base_url()),
                query
            )
        }
    }

    #[tokio::test]
    async fn send_posts_message_with_defaults() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/api/webhooks/webhook-id/test-token");
            then.status(204);
        });

        let service = DiscordFactory
            .build(&url::Url::parse(&discord_url(&server, "")).unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_username_config_independently() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/webhooks/webhook-id/test-token")
                .json_body(json!({
                    "username": "Alerts",
                    "avatar_url": default_avatar(),
                    "embeds": [
                        {
                            "title": default_title(),
                            "description": "hello",
                            "color": 0xFF9900
                        }
                    ]
                }));
            then.status(204);
        });

        let service = DiscordFactory
            .build(&url::Url::parse(&discord_url(&server, "username=Alerts")).unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_avatar_config_independently() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/webhooks/webhook-id/test-token")
                .json_body(json!({
                    "username": "Buzz",
                    "avatar_url": "https://example.com/avatar.png",
                    "embeds": [
                        {
                            "title": default_title(),
                            "description": "hello",
                            "color": 0xFF9900
                        }
                    ]
                }));
            then.status(204);
        });

        let service = DiscordFactory
            .build(&url::Url::parse(&discord_url(
                &server,
                "avatar=https%3A%2F%2Fexample.com%2Favatar.png",
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
                .path("/api/webhooks/webhook-id/test-token")
                .json_body(json!({
                    "username": "Buzz",
                    "avatar_url": default_avatar(),
                    "embeds": [
                        {
                            "title": "Deployment",
                            "description": "hello",
                            "color": 0xFF9900
                        }
                    ]
                }));
            then.status(204);
        });

        let service = DiscordFactory
            .build(&url::Url::parse(&discord_url(&server, "title=Deployment")).unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_color_config_independently() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/webhooks/webhook-id/test-token")
                .json_body(json!({
                    "username": "Buzz",
                    "avatar_url": default_avatar(),
                    "embeds": [
                        {
                            "title": default_title(),
                            "description": "hello",
                            "color": 0x00FF00
                        }
                    ]
                }));
            then.status(204);
        });

        let service = DiscordFactory
            .build(&url::Url::parse(&discord_url(&server, "color=00FF00")).unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_applies_combined_config() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/webhooks/webhook-id/test-token")
                .json_body(json!({
                    "username": "Ops",
                    "avatar_url": "https://example.com/avatar.png",
                    "embeds": [
                        {
                            "title": "Alert",
                            "description": "hello",
                            "color": 0x112233
                        }
                    ]
                }));
            then.status(204);
        });

        let service = DiscordFactory
            .build(&url::Url::parse(&discord_url(
                &server,
                "username=Ops&avatar=https%3A%2F%2Fexample.com%2Favatar.png&title=Alert&color=112233",
            ))
            .unwrap())
            .unwrap();

        service.send("hello").await.unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn send_returns_error_with_official_discord_error_payload() {
        let server = MockServer::start();
        let error_body = json!({
            "message": "401: Unauthorized",
            "code": 0
        });

        let _mock = server.mock(|when, then| {
            when.method(POST).path("/api/webhooks/webhook-id/test-token");
            then.status(401).json_body(error_body.clone());
        });

        let service = DiscordFactory
            .build(&url::Url::parse(&discord_url(&server, "")).unwrap())
            .unwrap();

        let err = service.send("hello").await.unwrap_err().to_string();
        assert!(err.contains("failed with status 401 Unauthorized"));
        assert!(err.contains("\"message\":\"401: Unauthorized\""));
        assert!(err.contains("\"code\":0"));
    }

    #[tokio::test]
    async fn send_fails_for_invalid_color() {
        let server = MockServer::start();
        let service = DiscordFactory
            .build(&url::Url::parse(&discord_url(&server, "color=ZZZZZZ")).unwrap())
            .unwrap();

        let err = service.send("hello").await.unwrap_err().to_string();
        assert!(err.contains("invalid digit"));
    }

    #[test]
    fn build_fails_when_webhook_id_missing() {
        let url = url::Url::parse("discord:///").unwrap();
        let err = match DiscordFactory.build(&url) {
            Ok(_) => panic!("expected build to fail"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("webhook id missing in discord URL"));
    }

    #[test]
    fn build_fails_when_token_missing() {
        let url = url::Url::parse("discord://@webhook").unwrap();
        let err = match DiscordFactory.build(&url) {
            Ok(_) => panic!("expected build to fail"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("webhook token missing in discord URL"));
    }
}
