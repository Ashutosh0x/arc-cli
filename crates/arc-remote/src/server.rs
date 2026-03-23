// SPDX-License-Identifier: MIT
//! Remote control server running within the main ARC CLI process.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{info, warn};

use crate::auth::auth_middleware;
use crate::protocol::{ClientMessage, ServerMessage, SessionStateDto};

#[derive(Clone)]
pub struct ServerConfig {
    pub bind_address: SocketAddr,
    pub secret_key: String,
}

pub struct RemoteServer {
    config: ServerConfig,
    app_state: Arc<AppState>,
}

pub struct AppState {
    /// Broadcasts outbound messages to all connected clients.
    pub tx_out: broadcast::Sender<ServerMessage>,
    /// Receives inbound commands from clients.
    pub tx_in: mpsc::Sender<ClientMessage>,
    /// Shared view of the current session state.
    pub current_state: RwLock<SessionStateDto>,
}

impl RemoteServer {
    pub fn new(config: ServerConfig) -> (Self, mpsc::Receiver<ClientMessage>) {
        let (tx_out, _) = broadcast::channel(100);
        let (tx_in, rx_in) = mpsc::channel(100);

        let app_state = Arc::new(AppState {
            tx_out,
            tx_in,
            current_state: RwLock::new(SessionStateDto::default()),
        });

        (Self { config, app_state }, rx_in)
    }

    /// Broadcast a state update to all connected remote clients.
    pub async fn broadcast_state(&self, state: SessionStateDto) {
        let mut current = self.app_state.current_state.write().await;
        *current = state.clone();
        let _ = self
            .app_state
            .tx_out
            .send(ServerMessage::StateUpdate(state));
    }

    /// Send a specific event (e.g., tool started, output chunk).
    pub fn broadcast_event(&self, msg: ServerMessage) {
        let _ = self.app_state.tx_out.send(msg);
    }

    /// Start the axum server in the background.
    pub async fn serve(self) -> Result<(), std::io::Error> {
        let secret = self.config.secret_key.clone();

        let app = Router::new()
            .route("/ws", get(ws_handler))
            .layer(axum::middleware::from_fn(move |req, next| {
                auth_middleware(req, next, secret.clone())
            }))
            .with_state(self.app_state);

        info!(
            "Remote control server listening on {}",
            self.config.bind_address
        );

        let listener = tokio::net::TcpListener::bind(&self.config.bind_address).await?;
        axum::serve(listener, app).await
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx_out.subscribe();

    // Send initial state
    let initial_state = state.current_state.read().await.clone();
    if let Ok(msg) = serde_json::to_string(&ServerMessage::StateUpdate(initial_state)) {
        let _ = socket.send(Message::Text(msg)).await;
    }

    loop {
        tokio::select! {
            // Outbound to client
            Ok(msg) = rx.recv() => {
                if let Ok(text) = serde_json::to_string(&msg) {
                    if socket.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
            // Inbound from client
            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        let _ = state.tx_in.send(client_msg).await;
                    } else {
                        warn!("Invalid client message: {}", text);
                    }
                }
            }
            else => break, // Socket closed
        }
    }
}
