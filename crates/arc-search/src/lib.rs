pub mod engine;
pub mod duckduckgo;
pub mod web_reader;

pub use engine::{SearchEngine, SearchResult};
pub use duckduckgo::DuckDuckGoScraper;
pub use web_reader::WebReader;
