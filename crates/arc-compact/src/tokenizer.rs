use anyhow::Result;
use tiktoken_rs::cl100k_base;

pub struct Tokenizer {
    bpe: tiktoken_rs::CoreBPE,
}

impl Tokenizer {
    pub fn new() -> Result<Self> {
        let bpe = cl100k_base()?;
        Ok(Self { bpe })
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        self.bpe.encode_with_special_tokens(text).len()
    }

    pub fn truncate(&self, text: &str, max_tokens: usize) -> String {
        let tokens = self.bpe.encode_with_special_tokens(text);
        if tokens.len() <= max_tokens {
            return text.to_string();
        }

        let truncated_tokens = &tokens[..max_tokens];
        self.bpe
            .decode(truncated_tokens.to_vec())
            .unwrap_or_else(|_| text.to_string())
    }
}
