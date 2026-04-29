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
                Box::new(services::discord::DiscordFactory),
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

#[doc(hidden)]
pub async fn __buzz_async(url: &str, msg: &str) {
    let buzz = Buzz::new();

    match buzz.build_service(url) {
        Ok(service) => {
            if let Err(e) = service.send(msg).await {
                eprintln!("Error sending notification. {}", e)
            }
        }
        Err(e) => eprintln!("error upon creation of service: {}", e),
    }
}

#[cfg(feature = "no_async")]
#[doc(hidden)]
pub fn __buzz_sync(url: &str, msg: &str) {
    match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime.block_on(__buzz_async(url, msg)),
        Err(e) => eprintln!("error creating tokio runtime: {}", e),
    }
}

#[macro_export]
macro_rules! buzz {
    ($url:expr, $msg:expr) => {{ $crate::__buzz_async($url, $msg).await }};
}

#[cfg(feature = "no_async")]
#[macro_export]
macro_rules! buzz_sync {
    ($url:expr, $msg:expr) => {{ $crate::__buzz_sync($url, $msg) }};
}
