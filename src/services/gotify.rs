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
            .into_iter()
            .next_back()
            .unwrap()
            .next_back()
            .unwrap()
            .to_owned();

        let domain = url
            .domain()
            .context("domain could not be parsed")?
            .to_owned();

        let config: GotifyConfig = serde_urlencoded::from_str(url.query().unwrap_or(""))
            .context("could not decode url query parameters")?;
        Ok(Box::new(GotifyService {
            domain,
            token,
            config,
        }))
    }
}
