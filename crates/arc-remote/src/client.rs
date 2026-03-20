//! Remote client: connects to a running ARC CLI session.

use crate::protocol::{ClientMessage, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use reqwest::header::{HeaderValue, AUTHORIZATION};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info};

pub struct RemoteClientConfig {
    pub server_url: String, // e.g. "ws://127.0.0.1:8080/ws"
    pub jwt_token: String,
}

pub struct RemoteClient {
    tx: mpsc::Sender<ClientMessage>,
}

impl RemoteClient {
    pub async fn connect(
        config: RemoteClientConfig,
        mut on_message: impl FnMut(ServerMessage) + Send + 'static,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut request = config.server_url.into_client_request()?;
        let headers = request.headers_mut();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.jwt_token))?,
        );

        let (ws_stream, _) = connect_async(request).await?;
        info!("Connected to remote session!");

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<ClientMessage>(100);

        // Spawn read loop
        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        on_message(server_msg);
                    }
                }
            }
            info!("Connection to remote session closed.");
        });

        // Spawn write loop
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(text) = serde_json::to_string(&msg) {
                    if write.send(Message::Text(text.into())).await.is_err() {
                        error!("Failed to send message to server");
                        break;
                    }
                }
            }
        });

        Ok(Self { tx })
    }

    pub async fn send(&self, msg: ClientMessage) {
        let _ = self.tx.send(msg).await;
    }
}
