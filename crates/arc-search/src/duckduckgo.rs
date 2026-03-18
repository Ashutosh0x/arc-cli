use crate::engine::{SearchEngine, SearchResult};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};

pub struct DuckDuckGoScraper {
    client: Client,
}

impl DuckDuckGoScraper {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

impl Default for DuckDuckGoScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchEngine for DuckDuckGoScraper {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
        
        let resp = self.client.get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .send()
            .await?
            .text()
            .await?;
            
        let document = Html::parse_document(&resp);
        // .result is the main div for results in html.duckduckgo.com
        let result_selector = Selector::parse(".result").unwrap();
        let title_selector = Selector::parse(".result__title .result__a").unwrap();
        let snippet_selector = Selector::parse(".result__snippet").unwrap();
        
        let mut results = Vec::new();
        
        for element in document.select(&result_selector).take(5) {
            let title_elem = element.select(&title_selector).next();
            let snippet_elem = element.select(&snippet_selector).next();
            
            if let (Some(t_el), Some(s_el)) = (title_elem, snippet_elem) {
                let title = t_el.text().collect::<Vec<_>>().join("");
                let url = t_el.value().attr("href").unwrap_or("").to_string();
                let snippet = s_el.text().collect::<Vec<_>>().join("");
                
                results.push(SearchResult { title, url, snippet });
            }
        }
        
        Ok(results)
    }
}
