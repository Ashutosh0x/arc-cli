// SPDX-License-Identifier: MIT
pub mod duckduckgo;
pub mod engine;
pub mod web_reader;

pub use duckduckgo::DuckDuckGoScraper;
pub use engine::{SearchEngine, SearchResult};
pub use web_reader::WebReader;
