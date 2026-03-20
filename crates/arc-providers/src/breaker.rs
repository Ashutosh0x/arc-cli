use std::time::{Duration, Instant};

pub enum CircuitState {
    Closed,   // Healthy, pass requests through
    Open,     // Tripped, reject all calls
    HalfOpen, // Testing one call to see if it succeeds
}

pub struct CircuitBreaker {
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub cooldown: Duration,
    pub last_failure: Option<Instant>,
    pub state: CircuitState,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown: Duration) -> Self {
        Self {
            failure_count: 0,
            failure_threshold,
            cooldown,
            last_failure: None,
            state: CircuitState::Closed,
        }
    }

    /// Check if a request is allowed to pass.
    pub fn is_allowed(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last) = self.last_failure {
                    if last.elapsed() >= self.cooldown {
                        self.state = CircuitState::HalfOpen;
                        return true; // Allow one probe
                    }
                }
                false
            },
            CircuitState::HalfOpen => false, // Already probing
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
        self.last_failure = None;
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        } else if matches!(self.state, CircuitState::HalfOpen) {
            // Failed during probe
            self.state = CircuitState::Open;
        }
    }
}
