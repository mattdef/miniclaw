//! Performance metrics tracking for the agent loop
//!
//! Tracks response times and calculates percentiles for monitoring.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

/// Target response time for 95th percentile (NFR-P4)
pub const TARGET_RESPONSE_TIME_MS: u128 = 2000;

/// Maximum number of response time samples to keep for percentile calculation
const MAX_SAMPLES: usize = 100;

/// Response time metrics tracker
pub struct ResponseMetrics {
    samples: Mutex<VecDeque<Duration>>,
}

impl ResponseMetrics {
    /// Creates a new response metrics tracker
    pub fn new() -> Self {
        Self {
            samples: Mutex::new(VecDeque::with_capacity(MAX_SAMPLES)),
        }
    }

    /// Records a response time sample
    pub fn record(&self, duration: Duration) {
        let mut samples = self.samples.lock().unwrap();

        // Add new sample
        samples.push_back(duration);

        // Keep only last MAX_SAMPLES
        if samples.len() > MAX_SAMPLES {
            samples.pop_front();
        }
    }

    /// Calculates the 95th percentile response time in milliseconds
    pub fn percentile_95(&self) -> Option<u128> {
        let samples = self.samples.lock().unwrap();

        if samples.is_empty() {
            return None;
        }

        // Convert to sorted vec of milliseconds
        let mut times_ms: Vec<u128> = samples.iter().map(|d| d.as_millis()).collect();

        times_ms.sort_unstable();

        // Calculate 95th percentile index
        let index = ((times_ms.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);

        Some(times_ms[index])
    }

    /// Returns the current sample count
    pub fn sample_count(&self) -> usize {
        self.samples.lock().unwrap().len()
    }

    /// Returns average response time in milliseconds
    pub fn average(&self) -> Option<u128> {
        let samples = self.samples.lock().unwrap();

        if samples.is_empty() {
            return None;
        }

        let sum: u128 = samples.iter().map(|d| d.as_millis()).sum();
        Some(sum / samples.len() as u128)
    }
}

impl Default for ResponseMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_calculation() {
        let metrics = ResponseMetrics::new();

        // Add 100 samples from 100ms to 10000ms
        for i in 1..=100 {
            metrics.record(Duration::from_millis(i * 100));
        }

        // 95th percentile of 100 samples should be at index 94 (95th value)
        // which is 9500ms
        let p95 = metrics.percentile_95().unwrap();
        assert_eq!(p95, 9500);
    }

    #[test]
    fn test_empty_metrics() {
        let metrics = ResponseMetrics::new();
        assert_eq!(metrics.percentile_95(), None);
        assert_eq!(metrics.average(), None);
        assert_eq!(metrics.sample_count(), 0);
    }

    #[test]
    fn test_single_sample() {
        let metrics = ResponseMetrics::new();
        metrics.record(Duration::from_millis(1500));

        assert_eq!(metrics.percentile_95(), Some(1500));
        assert_eq!(metrics.average(), Some(1500));
        assert_eq!(metrics.sample_count(), 1);
    }

    #[test]
    fn test_sample_limit() {
        let metrics = ResponseMetrics::new();

        // Add 150 samples (more than MAX_SAMPLES)
        for i in 1..=150 {
            metrics.record(Duration::from_millis(i * 10));
        }

        // Should only keep last 100
        assert_eq!(metrics.sample_count(), MAX_SAMPLES);
    }

    #[test]
    fn test_average_calculation() {
        let metrics = ResponseMetrics::new();

        metrics.record(Duration::from_millis(1000));
        metrics.record(Duration::from_millis(2000));
        metrics.record(Duration::from_millis(3000));

        assert_eq!(metrics.average(), Some(2000));
    }
}
