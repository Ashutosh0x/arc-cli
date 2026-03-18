use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use flume::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum PttEvent {
    Started,
    Stopped,
    Cancelled,
}

pub struct PushToTalkController {
    is_active: Arc<AtomicBool>,
    tx: Sender<PttEvent>,
}

impl PushToTalkController {
    pub fn new(tx: Sender<PttEvent>) -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            tx,
        }
    }

    /// Run the push-to-talk event loop (blocking, run in dedicated thread)
    /// Hold SPACE to talk, release to send. ESC to cancel.
    pub fn run_blocking(&self) -> anyhow::Result<()> {
        loop {
            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(' '),
                        kind: crossterm::event::KeyEventKind::Press,
                        ..
                    }) => {
                        if !self.is_active.load(Ordering::SeqCst) {
                            self.is_active.store(true, Ordering::SeqCst);
                            let _ = self.tx.send(PttEvent::Started);
                            debug!("PTT: Recording started (spacebar held)");
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(' '),
                        kind: crossterm::event::KeyEventKind::Release,
                        ..
                    }) => {
                        if self.is_active.load(Ordering::SeqCst) {
                            self.is_active.store(false, Ordering::SeqCst);
                            let _ = self.tx.send(PttEvent::Stopped);
                            debug!("PTT: Recording stopped (spacebar released)");
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Esc, ..
                    }) => {
                        self.is_active.store(false, Ordering::SeqCst);
                        let _ = self.tx.send(PttEvent::Cancelled);
                        break;
                    }
                    // Ctrl+C
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    }) => {
                        self.is_active.store(false, Ordering::SeqCst);
                        let _ = self.tx.send(PttEvent::Cancelled);
                        break;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }
}
