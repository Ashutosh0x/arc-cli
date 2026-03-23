// SPDX-License-Identifier: MIT
use crate::decoder::ImageDecoder;
use anyhow::Result;
use arc_providers::message::{Message, Role};
use arc_providers::traits::Provider;
use std::path::Path;
use std::sync::Arc;

pub struct VisionResult {
    pub description: String,
    pub recognized_text: Option<String>,
}

pub struct VisionAnalyzer {
    provider: Arc<dyn Provider>,
}

impl VisionAnalyzer {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    pub async fn analyze_image<P: AsRef<Path>>(
        &self,
        path: P,
        prompt: &str,
    ) -> Result<VisionResult> {
        let _base64_img = ImageDecoder::load_and_encode(path)?;

        // Mocking vision by supplying the prompt.
        // In a true implementation, we would attach the base64 content
        // using the Provider interface's vision capabilities.
        let modified_prompt = format!("[Attached Image Data - MOCKED]\n\n{}", prompt);
        let msg = Message {
            role: Role::User,
            content: modified_prompt,
            tool_calls: vec![],
            tool_call_id: None,
        };
        let analysis = self.provider.generate_text("default", &[msg]).await?;

        Ok(VisionResult {
            description: analysis,
            recognized_text: None,
        })
    }
}
