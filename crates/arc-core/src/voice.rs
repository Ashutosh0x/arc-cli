//! # Voice Mode — STT + Push-to-Talk Framework
//!
//! Framework for voice input: WebSocket audio streaming,
//! language detection, dev-term accuracy, modifier-key bindings.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoiceState {
    Idle,
    Listening,
    Processing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub language: String,
    pub push_to_talk_key: String,
    pub auto_detect_language: bool,
    pub sample_rate: u32,
    pub channels: u16,
    pub max_duration: Duration,
    pub silence_threshold_ms: u64,
    pub dev_term_boost: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            language: "en-US".into(),
            push_to_talk_key: "Ctrl+Shift+V".into(),
            auto_detect_language: true,
            sample_rate: 16000,
            channels: 1,
            max_duration: Duration::from_secs(30),
            silence_threshold_ms: 1500,
            dev_term_boost: true,
        }
    }
}

pub const SUPPORTED_LANGUAGES: &[&str] = &[
    "en-US", "en-GB", "es-ES", "fr-FR", "de-DE", "it-IT", "pt-BR", "ja-JP", "ko-KR", "zh-CN",
    "zh-TW", "ru-RU", "ar-SA", "hi-IN", "nl-NL", "pl-PL", "sv-SE", "da-DK", "fi-FI", "nb-NO",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: String,
    pub confidence: f64,
    pub duration_ms: u64,
    pub is_final: bool,
}

/// Voice engine abstraction — implementations would use platform-specific audio.
pub struct VoiceEngine {
    config: VoiceConfig,
    state: VoiceState,
    transcript_history: Vec<TranscriptionResult>,
}

impl VoiceEngine {
    pub fn new(config: VoiceConfig) -> Self {
        Self {
            config,
            state: VoiceState::Idle,
            transcript_history: Vec::new(),
        }
    }

    pub fn state(&self) -> VoiceState {
        self.state
    }

    /// Start listening (would trigger platform audio capture).
    pub fn start_listening(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            return Err("Voice mode not enabled".into());
        }
        self.state = VoiceState::Listening;
        Ok(())
    }

    /// Stop listening and process audio.
    pub fn stop_listening(&mut self) -> Result<(), String> {
        if self.state != VoiceState::Listening {
            return Err("Not listening".into());
        }
        self.state = VoiceState::Processing;
        Ok(())
    }

    /// Submit a transcription result (from STT backend).
    pub fn submit_transcription(&mut self, result: TranscriptionResult) {
        self.transcript_history.push(result);
        self.state = VoiceState::Idle;
    }

    /// Get the latest transcription.
    pub fn latest_transcription(&self) -> Option<&TranscriptionResult> {
        self.transcript_history.last()
    }

    pub fn history(&self) -> &[TranscriptionResult] {
        &self.transcript_history
    }

    pub fn clear_history(&mut self) {
        self.transcript_history.clear();
    }

    /// Check if platform audio is available.
    pub fn check_audio_support() -> AudioSupport {
        AudioSupport {
            #[cfg(target_os = "macos")]
            available: true,
            #[cfg(target_os = "windows")]
            available: true,
            #[cfg(target_os = "linux")]
            available: which::which("arecord").is_ok() || which::which("parecord").is_ok(),
            #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
            available: false,
            backend: Self::detect_backend(),
        }
    }

    fn detect_backend() -> String {
        #[cfg(target_os = "macos")]
        {
            return "CoreAudio".into();
        }
        #[cfg(target_os = "windows")]
        {
            return "WASAPI".into();
        }
        #[cfg(target_os = "linux")]
        {
            if which::which("parecord").is_ok() {
                return "PulseAudio".into();
            }
            if which::which("arecord").is_ok() {
                return "ALSA".into();
            }
            return "None".into();
        }
        #[allow(unreachable_code)]
        "Unknown".into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSupport {
    pub available: bool,
    pub backend: String,
}
