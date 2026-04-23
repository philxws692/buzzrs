use async_trait::async_trait;
use url::Url;

use anyhow::Result;

#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn send(&self, message: &str) -> Result<()>;
}

pub trait NotificationFactory: Send + Sync {
    fn scheme(&self) -> &'static str;

    fn build(&self, url: &Url) -> Result<Box<dyn NotificationService>>;
}
