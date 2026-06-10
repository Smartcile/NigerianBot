//! Minimal HTTP client for Servarr apps (Sonarr / Radarr), which share the same
//! v3 REST API shape: a base URL, an `X-Api-Key` header, and JSON everywhere.

use std::time::Duration;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone)]
pub struct Arr {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl Arr {
    pub fn new(http: reqwest::Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v3/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// GET with no query parameters.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_q(path, &[]).await
    }

    /// GET with query parameters (reqwest URL-encodes them).
    pub async fn get_q<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let resp = self
            .http
            .get(self.url(path))
            .header("X-Api-Key", &self.api_key)
            .query(query)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .context("could not reach the server")?
            .error_for_status()
            .context("the server returned an error")?;
        resp.json::<T>().await.context("unexpected response")
    }

    /// POST a JSON body and decode the JSON response.
    pub async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        let resp = self
            .http
            .post(self.url(path))
            .header("X-Api-Key", &self.api_key)
            .json(body)
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .context("could not reach the server")?
            .error_for_status()
            .context("the server returned an error")?;
        resp.json::<T>().await.context("unexpected response")
    }
}
