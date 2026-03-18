use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use flume::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

/// Raw PCM audio chunk
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp_ms: u64,
}

/// Captures audio from the default input device
pub struct AudioCapture {
    tx: Sender<AudioChunk>,
    rx: Receiver<AudioChunk>,
    is_recording: Arc<AtomicBool>,
    sample_rate: u32,
}

impl AudioCapture {
    pub fn new() -> Result<Self> {
        let (tx, rx) = flume::bounded(512);
        Ok(Self {
            tx,
            rx,
            is_recording: Arc::new(AtomicBool::new(false)),
            sample_rate: 16000,
        })
    }

    pub fn start(&self) -> Result<cpal::Stream> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No audio input device found")?;

        info!("Using audio device: {}", device.name()?);

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(1024),
        };

        let tx = self.tx.clone();
        let is_recording = self.is_recording.clone();
        let sample_rate = self.sample_rate;
        let start = std::time::Instant::now();

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::Relaxed) {
                    return;
                }
                let chunk = AudioChunk {
                    samples: data.to_vec(),
                    sample_rate,
                    channels: 1,
                    timestamp_ms: start.elapsed().as_millis() as u64,
                };
                let _ = tx.try_send(chunk);
            },
            |err| {
                warn!("Audio stream error: {err}");
            },
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }

    pub fn set_recording(&self, active: bool) {
        self.is_recording.store(active, Ordering::SeqCst);
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn receiver(&self) -> &Receiver<AudioChunk> {
        &self.rx
    }

    /// Collect all buffered audio into a single WAV byte buffer
    pub fn drain_to_wav(&self) -> Result<Vec<u8>> {
        let mut all_samples: Vec<f32> = Vec::new();
        while let Ok(chunk) = self.rx.try_recv() {
            all_samples.extend_from_slice(&chunk.samples);
        }

        let mut cursor = std::io::Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for sample in &all_samples {
            let s16 = (*sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(s16)?;
        }
        writer.finalize()?;
        Ok(cursor.into_inner())
    }
}
