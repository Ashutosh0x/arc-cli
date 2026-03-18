use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{info, warn};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioFormat {
    Pcm16,
    G711,
    Opus,
}

pub struct VoiceSession {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl VoiceSession {
    /// Send an audio PCM chunk to the LLM
    pub async fn send_audio(&mut self, data: Vec<u8>) -> Result<()> {
        let msg = Message::Binary(data.into());
        self.ws_stream.send(msg).await?;
        Ok(())
    }

    /// Read incoming audio chunks or control messages from the LLM
    pub async fn read_event(&mut self) -> Result<Option<Message>> {
        if let Some(msg) = self.ws_stream.next().await {
            let msg = msg?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
}

pub struct VoiceClient {
    endpoint: String,
    api_key: String,
}

impl VoiceClient {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self { endpoint, api_key }
    }

    pub async fn connect(&self) -> Result<VoiceSession> {
        let url = format!("{}?key={}", self.endpoint, self.api_key);
        let (ws_stream, _) = connect_async(&url).await?;

        info!("Connected to Realtime Voice API: {}", self.endpoint);

        Ok(VoiceSession { ws_stream })
    }
}
