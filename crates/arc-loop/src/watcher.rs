use anyhow::Result;
use notify::{Watcher, RecursiveMode, Event};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct DirWatcher {
    path: PathBuf,
}

impl DirWatcher {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }

    pub async fn watch(&self) -> Result<mpsc::Receiver<Event>> {
        let (tx, rx) = mpsc::channel(100);
        
        let path_clone = self.path.clone();
        
        tokio::spawn(async move {
            let (notify_tx, notify_rx) = std::sync::mpsc::channel();
            let mut watcher = notify::recommended_watcher(notify_tx).unwrap();
            
            if let Err(e) = watcher.watch(&path_clone, RecursiveMode::Recursive) {
                warn!("Failed to start watcher on {:?}: {}", path_clone, e);
                return;
            }
            
            info!("Started watching directory {:?}", path_clone);
            
            for res in notify_rx {
                match res {
                    Ok(event) => {
                        let _ = tx.send(event).await;
                    }
                    Err(e) => warn!("Watch error: {:?}", e),
                }
            }
        });
        
        Ok(rx)
    }
}
