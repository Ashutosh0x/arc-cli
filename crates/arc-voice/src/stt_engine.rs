use crate::languages::Language;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Speech-to-text backend selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SttBackend {
    /// OpenAI Whisper API
    Whisper { api_key: String, model: String },
    /// Deepgram streaming API
    Deepgram { api_key: String },
    /// Local Whisper.cpp via HTTP
    LocalWhisper { endpoint: String },
}

pub struct SttEngine {
    backend: SttBackend,
    client: Client,
    language: Language,
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    results: DeepgramResults,
}

#[derive(Debug, Deserialize)]
struct DeepgramResults {
    channels: Vec<DeepgramChannel>,
}

#[derive(Debug, Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
struct DeepgramAlternative {
    transcript: String,
    confidence: f64,
}

impl SttEngine {
    pub fn new(backend: SttBackend, language: Language) -> Self {
        Self {
            backend,
            client: Client::new(),
            language,
        }
    }

    /// Transcribe WAV audio bytes to text
    pub async fn transcribe(&self, wav_data: &[u8]) -> Result<String> {
        match &self.backend {
            SttBackend::Whisper { api_key, model } => {
                self.transcribe_whisper(wav_data, api_key, model).await
            },
            SttBackend::Deepgram { api_key } => self.transcribe_deepgram(wav_data, api_key).await,
            SttBackend::LocalWhisper { endpoint } => {
                self.transcribe_local(wav_data, endpoint).await
            },
        }
    }

    async fn transcribe_whisper(
        &self,
        wav_data: &[u8],
        api_key: &str,
        model: &str,
    ) -> Result<String> {
        let part = reqwest::multipart::Part::bytes(wav_data.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        let form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .text("language", self.language.code().to_string())
            .text("response_format", "json")
            .part("file", part);

        let resp = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {api_key}"))
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;

        let whisper: WhisperResponse = resp.json().await?;
        info!("Whisper transcription: {} chars", whisper.text.len());
        Ok(whisper.text)
    }

    async fn transcribe_deepgram(&self, wav_data: &[u8], api_key: &str) -> Result<String> {
        let url = format!(
            "https://api.deepgram.com/v1/listen?language={}&model=nova-2&smart_format=true",
            self.language.code()
        );

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {api_key}"))
            .header("Content-Type", "audio/wav")
            .body(wav_data.to_vec())
            .send()
            .await?
            .error_for_status()?;

        let dg: DeepgramResponse = resp.json().await?;
        let transcript = dg
            .results
            .channels
            .first()
            .and_then(|c| c.alternatives.first())
            .map(|a| a.transcript.clone())
            .unwrap_or_default();

        Ok(transcript)
    }

    async fn transcribe_local(&self, wav_data: &[u8], endpoint: &str) -> Result<String> {
        let part = reqwest::multipart::Part::bytes(wav_data.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        let form = reqwest::multipart::Form::new()
            .text("language", self.language.code().to_string())
            .part("file", part);

        let resp = self
            .client
            .post(format!("{endpoint}/inference"))
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;

        let body: serde_json::Value = resp.json().await?;
        Ok(body["text"].as_str().unwrap_or("").to_string())
    }
}
