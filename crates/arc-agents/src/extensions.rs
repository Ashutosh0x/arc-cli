// SPDX-License-Identifier: MIT
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct AgentMailbox {
    messages: VecDeque<String>,
}

pub struct PeerNetwork {
    peers: HashMap<String, mpsc::Sender<String>>,
}

pub struct AgentTeam {
    pub name: String,
    pub members: Vec<String>,
}

pub struct ThinkingSpinner {
    pub phase: String,
}

impl Default for AgentMailbox {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentMailbox {
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
        }
    }
    pub fn send(&mut self, msg: String) {
        self.messages.push_back(msg);
    }
    pub fn receive(&mut self) -> Option<String> {
        self.messages.pop_front()
    }
}

impl Default for PeerNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerNetwork {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: String, tx: mpsc::Sender<String>) {
        info!("Registering peer {} in PeerNetwork", id);
        self.peers.insert(id, tx);
    }

    pub async fn broadcast(&self, msg: &str) {
        for (id, tx) in &self.peers {
            if let Err(e) = tx.send(msg.to_string()).await {
                warn!("Failed to broadcast to peer {}: {}", id, e);
            }
        }
    }
}
