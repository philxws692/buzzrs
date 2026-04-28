use anyhow::{Result, anyhow};
use url::Url;

use crate::core::{NotificationFactory, NotificationService};

pub(crate) mod core;

mod services;

pub struct Buzz {
    factories: Vec<Box<dyn NotificationFactory>>,
}

impl Buzz {
    pub fn new() -> Self {
        Self {
            factories: vec![
                Box::new(services::gotify::GotifyFactory),
                Box::new(services::ntfy::NtfyFactory),
            ],
        }
    }

    pub fn build_service(&self, url_str: &str) -> Result<Box<dyn NotificationService>> {
        let url = Url::parse(url_str)?;
        let factory = self
            .factories
            .iter()
            .find(|f| f.scheme() == url.scheme())
            .ok_or_else(|| anyhow!("scheme '{}' not supported", url.scheme()))?;

        factory.build(&url)
    }
}

impl Default for Buzz {
    fn default() -> Self {
        Self::new()
    }
}

#[macro_export]
macro_rules! buzz {
    ($url:expr, $msg:expr) => {{
        let buzz = $crate::Buzz::new();

        match buzz.build_service($url) {
            Ok(service) => {
                if let Err(e) = service.send($msg).await {
                    eprintln!("Error sending notification. {}", e)
                }
            }
            Err(e) => eprintln!("error upon creation of service: {}", e),
        }
    }};
}
