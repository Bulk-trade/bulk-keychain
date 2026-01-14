//! Nonce management utilities
//!
//! The BULK exchange requires unique nonces for replay protection.
//! This module provides helpers for generating and managing nonces.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Strategy for generating nonces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceStrategy {
    /// Use current timestamp in milliseconds
    Timestamp,
    /// Use an incrementing counter
    Counter,
    /// Timestamp with sub-millisecond counter for high-frequency
    TimestampWithCounter,
}

/// Thread-safe nonce manager
pub struct NonceManager {
    strategy: NonceStrategy,
    counter: AtomicU64,
    last_timestamp: AtomicU64,
}

impl NonceManager {
    /// Create a new nonce manager with the specified strategy
    pub fn new(strategy: NonceStrategy) -> Self {
        Self {
            strategy,
            counter: AtomicU64::new(0),
            last_timestamp: AtomicU64::new(0),
        }
    }

    /// Create a timestamp-based nonce manager
    pub fn timestamp() -> Self {
        Self::new(NonceStrategy::Timestamp)
    }

    /// Create a counter-based nonce manager
    pub fn counter() -> Self {
        Self::new(NonceStrategy::Counter)
    }

    /// Create a high-frequency nonce manager (timestamp + counter)
    pub fn high_frequency() -> Self {
        Self::new(NonceStrategy::TimestampWithCounter)
    }

    /// Get the next nonce
    pub fn next(&self) -> u64 {
        match self.strategy {
            NonceStrategy::Timestamp => current_timestamp_millis(),
            NonceStrategy::Counter => self.counter.fetch_add(1, Ordering::SeqCst),
            NonceStrategy::TimestampWithCounter => self.next_hf(),
        }
    }

    /// High-frequency nonce: ensures strictly increasing values
    /// Uses fetch_add to guarantee uniqueness across concurrent calls
    fn next_hf(&self) -> u64 {
        // Simply use an atomic counter that combines timestamp with sequence
        // This guarantees uniqueness and strict ordering
        let base = current_timestamp_millis() * 1000; // Leave room for 1000 nonces per millisecond
        let seq = self.counter.fetch_add(1, Ordering::SeqCst);
        base + seq
    }

    /// Reset the counter (useful for testing)
    pub fn reset(&self) {
        self.counter.store(0, Ordering::SeqCst);
        self.last_timestamp.store(0, Ordering::SeqCst);
    }
}

impl Default for NonceManager {
    fn default() -> Self {
        Self::timestamp()
    }
}

/// Get current timestamp in milliseconds
#[inline]
pub fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64
}

/// Get current timestamp in microseconds
#[inline]
pub fn current_timestamp_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_micros() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_nonce() {
        let manager = NonceManager::timestamp();
        let n1 = manager.next();
        let n2 = manager.next();

        // Should be close to current time
        let now = current_timestamp_millis();
        assert!(n1 <= now && n1 > now - 1000);

        // Timestamps should be non-decreasing
        assert!(n2 >= n1);
    }

    #[test]
    fn test_counter_nonce() {
        let manager = NonceManager::counter();
        assert_eq!(manager.next(), 0);
        assert_eq!(manager.next(), 1);
        assert_eq!(manager.next(), 2);
    }

    #[test]
    fn test_high_frequency_nonce() {
        let manager = NonceManager::high_frequency();

        // Generate many nonces quickly
        let nonces: Vec<_> = (0..100).map(|_| manager.next()).collect();

        // All should be strictly increasing
        for i in 1..nonces.len() {
            assert!(
                nonces[i] > nonces[i - 1],
                "Nonce {} ({}) should be greater than {} ({})",
                i,
                nonces[i],
                i - 1,
                nonces[i - 1]
            );
        }
    }
}
