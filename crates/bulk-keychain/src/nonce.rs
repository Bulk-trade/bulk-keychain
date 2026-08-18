//! Nonce management utilities
//!
//! The BULK exchange requires unique nonces for replay protection.
//! This module provides helpers for generating and managing nonces.

use serde::{Deserialize, Deserializer, Serializer};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Error, Result};

static LAST_TIMESTAMP_NANOS: AtomicU64 = AtomicU64::new(0);

/// Strategy for generating nonces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceStrategy {
    /// Use current Unix timestamp in nanoseconds
    Timestamp,
    /// Use an incrementing counter
    Counter,
    /// Nanosecond timestamp with monotonic collision handling
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
            NonceStrategy::Timestamp => current_timestamp_nanos(),
            NonceStrategy::Counter => self.counter.fetch_add(1, Ordering::SeqCst),
            NonceStrategy::TimestampWithCounter => self.next_hf(),
        }
    }

    /// High-frequency nonce: ensures strictly increasing values
    /// while remaining based on Unix nanoseconds.
    fn next_hf(&self) -> u64 {
        let now = current_timestamp_nanos();
        let mut previous = self.last_timestamp.load(Ordering::Acquire);

        loop {
            let next = now.max(previous.saturating_add(1));
            match self.last_timestamp.compare_exchange_weak(
                previous,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return next,
                Err(actual) => previous = actual,
            }
        }
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
    #[cfg(target_arch = "wasm32")]
    {
        // Date.now() is an integer well inside JavaScript's exact integer range.
        // Convert it to u64 before scaling so the nonce itself never uses f64.
        return js_sys::Date::now() as u64;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis() as u64
    }
}

/// Get current timestamp in microseconds
#[inline]
pub fn current_timestamp_micros() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        return current_timestamp_millis()
            .checked_mul(1_000)
            .expect("Unix timestamp in microseconds exceeds u64");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_micros() as u64
    }
}

fn monotonic_timestamp_nanos(candidate: u64, last: &AtomicU64) -> u64 {
    let mut previous = last.load(Ordering::Acquire);

    loop {
        let next = candidate.max(previous.saturating_add(1));
        match last.compare_exchange_weak(previous, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return next,
            Err(actual) => previous = actual,
        }
    }
}

/// Get the current Unix timestamp in nanoseconds, monotonically incrementing
/// when the platform clock cannot distinguish consecutive calls.
#[inline]
pub fn current_timestamp_nanos() -> u64 {
    #[cfg(target_arch = "wasm32")]
    let candidate = current_timestamp_millis()
        .checked_mul(1_000_000)
        .expect("Unix timestamp in nanoseconds exceeds u64");

    #[cfg(not(target_arch = "wasm32"))]
    let candidate = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos()
        .try_into()
        .expect("Unix timestamp in nanoseconds exceeds u64");

    monotonic_timestamp_nanos(candidate, &LAST_TIMESTAMP_NANOS)
}

/// Parse a transaction nonce from its exact decimal-string representation.
pub fn parse_decimal(value: &str) -> Result<u64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::InvalidNonce(value.to_owned()));
    }

    value
        .parse::<u64>()
        .map_err(|_| Error::InvalidNonce(value.to_owned()))
}

/// Serde adapter that represents a `u64` nonce as a decimal string.
pub mod serde_decimal {
    use super::*;

    pub fn serialize<S>(value: &u64, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse_decimal(&value).map_err(serde::de::Error::custom)
    }
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
        let now = current_timestamp_nanos();
        assert!(n1 <= now && n1 > now - 1_000_000_000);

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

    #[test]
    fn test_nanosecond_clock_handles_same_tick_monotonically() {
        let last = AtomicU64::new(0);
        assert_eq!(monotonic_timestamp_nanos(123, &last), 123);
        assert_eq!(monotonic_timestamp_nanos(123, &last), 124);
        assert_eq!(monotonic_timestamp_nanos(122, &last), 125);
    }

    #[test]
    fn test_parse_decimal_above_js_safe_integer() {
        assert_eq!(
            parse_decimal("9007199254740993").unwrap(),
            9_007_199_254_740_993
        );
        assert_eq!(parse_decimal(&u64::MAX.to_string()).unwrap(), u64::MAX);
        assert!(parse_decimal("18446744073709551616").is_err());
        assert!(parse_decimal("1.0").is_err());
        assert!(parse_decimal("-1").is_err());
        assert!(parse_decimal("").is_err());
    }
}
