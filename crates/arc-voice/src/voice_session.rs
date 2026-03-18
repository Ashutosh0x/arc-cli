use crate::audio_capture::AudioCapture;
use crate::languages::Language;
use crate::push_to_talk::{PttEvent, PushToTalkController};
use crate::stt_engine::{SttBackend, SttEngine};
use anyhow::Result;
use flume::{Receiver, Sender};
use std::sync::Arc;
use tracing::{error, info};

pub struct VoiceSession {
    engine: Arc<SttEngine>,
    capture: Arc<AudioCapture>,
    language: Language,
    transcript_tx: Sender<String>,
    transcript_rx: Receiver<String>,
}

impl VoiceSession {
    pub fn new(backend: SttBackend, language: Language) -> Result<Self> {
        let (transcript_tx, transcript_rx) = flume::bounded(64);
        Ok(Self {
            engine: Arc::new(SttEngine::new(backend, language)),
            capture: Arc::new(AudioCapture::new()?),
            language,
            transcript_tx,
            transcript_rx,
        })
    }

    /// Start the voice session — runs PTT loop and transcription pipeline
    pub async fn start(&self) -> Result<()> {
        let _stream = self.capture.start()?;
        let (ptt_tx, ptt_rx) = flume::bounded(32);
        let ptt = PushToTalkController::new(ptt_tx);

        // PTT thread (blocking keyboard events)
        let ptt_handle = std::thread::spawn(move || {
            ptt.run_blocking()
        });

        // Transcription loop
        let capture = self.capture.clone();
        let engine = self.engine.clone();
        let tx = self.transcript_tx.clone();

        let transcription_handle = tokio::spawn(async move {
            while let Ok(event) = ptt_rx.recv_async().await {
                match event {
                    PttEvent::Started => {
                        capture.set_recording(true);
                        info!("🎤 Recording... (release spacebar to send)");
                    }
                    PttEvent::Stopped => {
                        capture.set_recording(false);
                        info!("⏹️  Processing speech...");

                        match capture.drain_to_wav() {
                            Ok(wav_data) if wav_data.len() > 44 => {
                                match engine.transcribe(&wav_data).await {
                                    Ok(text) if !text.trim().is_empty() => {
                                        info!("📝 Transcribed: {text}");
                                        let _ = tx.send_async(text).await;
                                    }
                                    Ok(_) => {
                                        info!("(no speech detected)");
                                    }
                                    Err(e) => {
                                        error!("Transcription error: {e}");
                                    }
                                }
                            }
                            Ok(_) => info!("(recording too short)"),
                            Err(e) => error!("WAV encoding error: {e}"),
                        }
                    }
                    PttEvent::Cancelled => {
                        capture.set_recording(false);
                        info!("Voice mode cancelled");
                        break;
                    }
                }
            }
        });

        transcription_handle.await?;
        let _ = ptt_handle.join();
        Ok(())
    }

    /// Get the next transcribed text (used by REPL to feed into agent)
    pub async fn next_transcript(&self) -> Option<String> {
        self.transcript_rx.recv_async().await.ok()
    }

    pub fn transcript_receiver(&self) -> Receiver<String> {
        self.transcript_rx.clone()
    }
}
