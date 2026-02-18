//! Circuit breaker pattern for external service fault tolerance
//!
//! Prevents cascading failures by temporarily disabling calls to failing services.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed - requests flow normally
    Closed,
    /// Circuit is open - requests are rejected immediately
    Open,
    /// Circuit is half-open - test requests are allowed
    HalfOpen,
}

/// Circuit breaker for protecting external service calls
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Current state
    state: Arc<RwLock<CircuitState>>,
    /// Failure count in current window
    failure_count: Arc<RwLock<u32>>,
    /// Time when circuit was opened
    opened_at: Arc<RwLock<Option<Instant>>>,
    /// Failure threshold before opening
    failure_threshold: u32,
    /// Duration to keep circuit open
    timeout: Duration,
    /// Service name for logging
    service_name: String,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    ///
    /// # Arguments
    /// * `service_name` - Name of the service being protected
    /// * `failure_threshold` - Number of failures before opening circuit
    /// * `timeout` - Duration to keep circuit open before trying again
    pub fn new(service_name: impl Into<String>, failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            opened_at: Arc::new(RwLock::new(None)),
            failure_threshold,
            timeout,
            service_name: service_name.into(),
        }
    }

    /// Check if a call should be allowed
    pub async fn can_call(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(opened_at) = *self.opened_at.read().await {
                    if opened_at.elapsed() >= self.timeout {
                        // Transition to half-open
                        *self.state.write().await = CircuitState::HalfOpen;
                        debug!(
                            service = %self.service_name,
                            "Circuit breaker transitioning to HalfOpen"
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful call
    pub async fn record_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::HalfOpen => {
                // Successful test call - close the circuit
                *self.state.write().await = CircuitState::Closed;
                *self.failure_count.write().await = 0;
                *self.opened_at.write().await = None;
                debug!(
                    service = %self.service_name,
                    "Circuit breaker closed after successful test"
                );
            }
            CircuitState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                *self.failure_count.write().await = 0;
            }
        }
    }

    /// Record a failed call
    pub async fn record_failure(&self) {
        let mut count = self.failure_count.write().await;
        *count += 1;

        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                if *count >= self.failure_threshold {
                    // Open the circuit
                    *self.state.write().await = CircuitState::Open;
                    *self.opened_at.write().await = Some(Instant::now());
                    warn!(
                        service = %self.service_name,
                        failures = *count,
                        threshold = self.failure_threshold,
                        "Circuit breaker opened due to failures"
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Test call failed - reopen the circuit
                *self.state.write().await = CircuitState::Open;
                *self.opened_at.write().await = Some(Instant::now());
                warn!(
                    service = %self.service_name,
                    "Circuit breaker reopened after failed test"
                );
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get current failure count
    pub async fn failure_count(&self) -> u32 {
        *self.failure_count.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::new("test_service", 3, Duration::from_millis(100));

        // Initially closed
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.can_call().await);

        // Record failures
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Closed);

        cb.record_failure().await;
        // Should be open now
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(!cb.can_call().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let cb = CircuitBreaker::new("test_service", 2, Duration::from_millis(50));

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for timeout
        sleep(Duration::from_millis(60)).await;

        // Should allow a test call (half-open)
        assert!(cb.can_call().await);
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closes_after_success() {
        let cb = CircuitBreaker::new("test_service", 2, Duration::from_millis(50));

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait and transition to half-open
        sleep(Duration::from_millis(60)).await;
        assert!(cb.can_call().await);

        // Record success - should close
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert_eq!(cb.failure_count().await, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reopens_on_half_open_failure() {
        let cb = CircuitBreaker::new("test_service", 2, Duration::from_millis(50));

        // Open the circuit
        cb.record_failure().await;
        cb.record_failure().await;

        // Wait and transition to half-open
        sleep(Duration::from_millis(60)).await;
        assert!(cb.can_call().await);

        // Record another failure - should reopen
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
    }
}
