use async_trait::async_trait;
use serde::Deserialize;

use anyhow::{Context, Result, anyhow};
use serde_json::json;

use crate::core::{NotificationFactory, NotificationService};

#[derive(Deserialize, Debug)]
pub struct NtfyConfig {
    /// The url of a notification attachment [more info](https://docs.ntfy.sh/publish/#attach-file-from-a-url)
    #[serde(default)]
    attach: String,
    #[serde(default = "default_cache")]
    cache: bool,
    #[serde(default)]
    click: String,
    #[serde(default, alias = "in", alias = "at")]
    delay: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    priority: u8,
    #[serde(default)]
    disable_tls: bool,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    title: String,
}

pub struct NtfyService {
    domain: String,
    token: String,
    topic: String,
    config: NtfyConfig,
}

fn default_cache() -> bool {
    true
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
        let token = url
            .path_segments()
            .into_iter()
            .next()
            .unwrap()
            .next()
            .unwrap()
            .to_owned();

        let topic = url
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
