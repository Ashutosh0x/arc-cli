use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

use crate::error::{ArcError, Result};
use arc_providers::message::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub total_input_tokens: u64,
    #[serde(default)]
    pub total_output_tokens: u64,
    #[serde(default)]
    pub total_cost_usd: f64,
}

impl Session {
    pub fn new(name: Option<String>) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id: id.clone(),
            name: name.unwrap_or_else(|| format!("Session {}", &id[..8])),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            messages: Vec::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    pub fn record_metrics(&mut self, input: u64, output: u64, cost: f64) {
        self.total_input_tokens += input;
        self.total_output_tokens += output;
        self.total_cost_usd += cost;
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    fn sessions_dir() -> Result<PathBuf> {
        let dir = dirs::home_dir()
            .ok_or(ArcError::NoHomeDir)?
            .join(".arc")
            .join("sessions");
        Ok(dir)
    }

    pub async fn save(&self) -> Result<()> {
        let dir = Self::sessions_dir()?;
        if !dir.exists() {
            fs::create_dir_all(&dir).await?;
        }
        
        let path = dir.join(format!("{}.json", self.id));
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content).await?;
        
        // Also update a special symlink/pointer to "latest" session
        let latest = dir.join("latest.txt");
        fs::write(latest, &self.id).await?;
        
        Ok(())
    }

    pub async fn load(id: &str) -> Result<Self> {
        let path = Self::sessions_dir()?.join(format!("{}.json", id));
        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| ArcError::Session(format!("Failed to read session file: {e}")))?;
        let session = serde_json::from_str(&content)?;
        Ok(session)
    }

    pub async fn new_or_resume(_config: &crate::config::Config) -> Result<Self> {
        // Simple logic: check if there's a recent session, otherwise start fresh
        let dir = match Self::sessions_dir() {
            Ok(d) => d,
            Err(_) => return Ok(Self::new(None)),
        };
        
        let latest = dir.join("latest.txt");
        if latest.exists() {
            if let Ok(id) = fs::read_to_string(&latest).await {
                if let Ok(session) = Self::load(id.trim()).await {
                    return Ok(session);
                }
            }
        }
        
        Ok(Self::new(None))
    }
}

