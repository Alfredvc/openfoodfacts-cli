use anyhow::{bail, Context, Result};
use serde_json::Value;

const DEFAULT_BASE_URL: &str = "https://world.openfoodfacts.net";

pub struct Client {
    inner: reqwest::Client,
    pub base_url: String,
}

impl Client {
    pub fn new() -> Result<Self> {
        let base_url = std::env::var("OFF_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let base_url = base_url.trim_end_matches('/').to_string();
        let user_agent = format!(
            "openfoodfacts-cli/{} (https://github.com/alfredvc/openfoodfacts-cli)",
            env!("CARGO_PKG_VERSION")
        );
        let inner = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { inner, base_url })
    }

    pub async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .inner
            .get(&url)
            .query(params)
            .send()
            .await
            .with_context(|| format!("request to {url} failed"))?;

        let status = response.status();
        if status == 429 || status == 403 {
            bail!("rate limit exceeded");
        }
        if status == 404 {
            bail!("not found: {path}");
        }
        if !status.is_success() {
            bail!("API error: HTTP {status}");
        }

        response.json::<Value>().await.context("failed to parse JSON response")
    }
}
