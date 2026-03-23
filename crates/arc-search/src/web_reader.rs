// SPDX-License-Identifier: MIT
use anyhow::{Result, anyhow};
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;

pub struct WebReader {
    client: Client,
}

impl WebReader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn read_page(&self, url: &str) -> Result<String> {
        info!("Reading web page: {}", url);
        let resp = self
            .client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .send()
            .await?
            .text()
            .await?;

        let document = Html::parse_document(&resp);
        let mut content = String::new();

        // Very basic extraction: grab text from p, h1, h2, h3, li
        let selectors = ["p", "h1", "h2", "h3", "li"];
        for s in selectors {
            let selector = Selector::parse(s).unwrap();
            for element in document.select(&selector) {
                let text: String = element.text().collect::<Vec<_>>().join(" ");
                if !text.trim().is_empty() {
                    content.push_str(&text);
                    content.push('\n');
                }
            }
        }

        if content.is_empty() {
            return Err(anyhow!("No readable content found on page"));
        }

        Ok(content)
    }
}

impl Default for WebReader {
    fn default() -> Self {
        Self::new()
    }
}
