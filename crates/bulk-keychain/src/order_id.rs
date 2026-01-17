//! Order ID computation
//!
//! Computes deterministic order IDs that match BULK.
//!
//! ## Algorithm
//!
//! Order IDs use a custom serialization format with fixed-point numbers:
//! - Floats are converted to u64 via: `(value × 10^8).round()`
//! - Strings use u32 length prefix (not u64)
//! - SHA256 hash of the serialized bytes
//!
//! This ensures deterministic IDs across platforms despite float precision differences.

use crate::types::*;
use sha2::{Digest, Sha256};

/// Fixed-point multiplier: 10^8 (8 decimal places)
const DECIMALS_MULTIPLIER: f64 = 100_000_000.0;

/// Convert f64 to fixed-point u64
///
/// `(value × 10^8).round()` ensures deterministic results regardless of
/// floating-point representation (e.g., 0.917 and 0.91700000000000004 both → 91700000)
#[inline]
fn to_fixed_point(value: f64) -> u64 {
    (value * DECIMALS_MULTIPLIER).round() as u64
}

/// Write a string with u32 length prefix (wincode-style for OID)
#[inline]
fn write_string(buffer: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buffer.extend_from_slice(bytes);
}

/// Compute order ID for a limit order
///
/// Serialization format:
/// - nonce: u64 LE (8 bytes)
/// - market: u32 length + UTF-8 (4 + len bytes)
/// - owner: raw 32 bytes
/// - side: u8 (0=Buy, 1=Sell)
/// - amount: fixed-point u64 LE (8 bytes)
/// - price: fixed-point u64 LE (8 bytes)
/// - tif: u32 enum (4 bytes)
/// - reduce_only: u8 (0/1)
#[inline]
pub fn compute_limit_order_id(
    nonce: u64,
    market: &str,
    owner: &Pubkey,
    is_buy: bool,
    amount: f64,
    price: f64,
    tif: TimeInForce,
    reduce_only: bool,
) -> Hash {
    let mut buffer = Vec::with_capacity(128);

    // nonce: u64 LE
    buffer.extend_from_slice(&nonce.to_le_bytes());

    // market: u32 length + UTF-8
    write_string(&mut buffer, market);

    // owner: raw 32 bytes
    buffer.extend_from_slice(owner.as_bytes());

    // side: u8 (0=Buy, 1=Sell)
    buffer.push(if is_buy { 0 } else { 1 });

    // amount: fixed-point u64 LE
    buffer.extend_from_slice(&to_fixed_point(amount).to_le_bytes());

    // price: fixed-point u64 LE
    buffer.extend_from_slice(&to_fixed_point(price).to_le_bytes());

    // tif: u32 enum (GTC=0, IOC=1, ALO=2)
    buffer.extend_from_slice(&tif.discriminant().to_le_bytes());

    // reduce_only: u8
    buffer.push(if reduce_only { 1 } else { 0 });

    // SHA256
    let hash: [u8; 32] = Sha256::digest(&buffer).into();
    Hash::from_bytes(hash)
}

/// Compute order ID for a market order
///
/// Serialization format:
/// - nonce: u64 LE (8 bytes)
/// - market: u32 length + UTF-8 (4 + len bytes)
/// - owner: raw 32 bytes
/// - side: u8 (0=Buy, 1=Sell)
/// - amount: fixed-point u64 LE (8 bytes)
/// - reduce_only: u8 (0/1)
#[inline]
pub fn compute_market_order_id(
    nonce: u64,
    market: &str,
    owner: &Pubkey,
    is_buy: bool,
    amount: f64,
    reduce_only: bool,
) -> Hash {
    let mut buffer = Vec::with_capacity(96);

    // nonce: u64 LE
    buffer.extend_from_slice(&nonce.to_le_bytes());

    // market: u32 length + UTF-8
    write_string(&mut buffer, market);

    // owner: raw 32 bytes
    buffer.extend_from_slice(owner.as_bytes());

    // side: u8 (0=Buy, 1=Sell)
    buffer.push(if is_buy { 0 } else { 1 });

    // amount: fixed-point u64 LE
    buffer.extend_from_slice(&to_fixed_point(amount).to_le_bytes());

    // reduce_only: u8
    buffer.push(if reduce_only { 1 } else { 0 });

    // SHA256
    let hash: [u8; 32] = Sha256::digest(&buffer).into();
    Hash::from_bytes(hash)
}

/// Compute order ID for an Order (auto-detects limit vs market)
///
/// Returns `Some(Hash)` for Order items, `None` for Cancel/CancelAll
pub fn compute_order_item_id(item: &OrderItem, nonce: u64, owner: &Pubkey) -> Option<Hash> {
    match item {
        OrderItem::Order(order) => Some(compute_order_id(order, nonce, owner)),
        OrderItem::Cancel(_) => None,
        OrderItem::CancelAll(_) => None,
    }
}

/// Compute order ID for an Order
#[inline]
pub fn compute_order_id(order: &Order, nonce: u64, owner: &Pubkey) -> Hash {
    match &order.order_type {
        OrderType::Limit { tif } => compute_limit_order_id(
            nonce,
            &order.symbol,
            owner,
            order.is_buy,
            order.size,
            order.price,
            *tif,
            order.reduce_only,
        ),
        OrderType::Trigger { .. } => compute_market_order_id(
            nonce,
            &order.symbol,
            owner,
            order.is_buy,
            order.size,
            order.reduce_only,
        ),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_point_conversion() {
        // Basic conversions
        assert_eq!(to_fixed_point(1.0), 100_000_000);
        assert_eq!(to_fixed_point(0.1), 10_000_000);
        assert_eq!(to_fixed_point(100000.0), 10_000_000_000_000);

        // The key test: float precision issues should produce same result
        // 0.917 and 0.91700000000000004 should both round to 91700000
        assert_eq!(to_fixed_point(0.917), 91_700_000);
        assert_eq!(to_fixed_point(0.91700000000000004), 91_700_000);

        // Edge cases
        assert_eq!(to_fixed_point(0.0), 0);
        assert_eq!(to_fixed_point(0.00000001), 1); // 1 satoshi
    }

    #[test]
    fn test_limit_order_id_deterministic() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let nonce = 1234567890u64;

        let id1 = compute_limit_order_id(
            nonce,
            "BTC-USD",
            &owner,
            true, // buy
            0.1,
            100000.0,
            TimeInForce::Gtc,
            false,
        );

        let id2 = compute_limit_order_id(
            nonce,
            "BTC-USD",
            &owner,
            true,
            0.1,
            100000.0,
            TimeInForce::Gtc,
            false,
        );

        // Same inputs = same ID
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_limit_order_id_unique_per_nonce() {
        let owner = Pubkey::from_bytes([1u8; 32]);

        let id1 = compute_limit_order_id(
            1,
            "BTC-USD",
            &owner,
            true,
            0.1,
            100000.0,
            TimeInForce::Gtc,
            false,
        );

        let id2 = compute_limit_order_id(
            2,
            "BTC-USD",
            &owner,
            true,
            0.1,
            100000.0,
            TimeInForce::Gtc,
            false,
        );

        // Different nonces = different IDs
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_limit_vs_market_different() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let nonce = 1234567890u64;

        let limit_id = compute_limit_order_id(
            nonce,
            "BTC-USD",
            &owner,
            true,
            0.1,
            100000.0,
            TimeInForce::Gtc,
            false,
        );

        let market_id = compute_market_order_id(
            nonce,
            "BTC-USD",
            &owner,
            true,
            0.1,
            false,
        );

        // Different order types = different IDs
        assert_ne!(limit_id, market_id);
    }

    #[test]
    fn test_float_precision_determinism() {
        let owner = Pubkey::from_bytes([42u8; 32]);
        let nonce = 9999u64;

        // These might be represented differently in memory due to float precision
        let price1 = 0.917;
        let price2 = 0.91700000000000004;
        let amount1 = 1.234;
        let amount2 = 1.2340000000000002;

        let id1 = compute_limit_order_id(
            nonce,
            "ETH-USD",
            &owner,
            false,
            amount1,
            price1,
            TimeInForce::Ioc,
            true,
        );

        let id2 = compute_limit_order_id(
            nonce,
            "ETH-USD",
            &owner,
            false,
            amount2,
            price2,
            TimeInForce::Ioc,
            true,
        );

        // Fixed-point conversion ensures same hash!
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_market_order_id_deterministic() {
        let owner = Pubkey::from_bytes([7u8; 32]);
        let nonce = 5555u64;

        let id1 = compute_market_order_id(nonce, "SOL-USD", &owner, true, 10.0, false);
        let id2 = compute_market_order_id(nonce, "SOL-USD", &owner, true, 10.0, false);

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_compute_order_id_limit() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);

        let id = compute_order_id(&order, 1234567890, &owner);

        // Should match direct limit computation
        let expected = compute_limit_order_id(
            1234567890,
            "BTC-USD",
            &owner,
            true,
            0.1,
            100000.0,
            TimeInForce::Gtc,
            false,
        );

        assert_eq!(id, expected);
    }

    #[test]
    fn test_compute_order_id_market() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let order = Order::market("BTC-USD", true, 0.1);

        let id = compute_order_id(&order, 1234567890, &owner);

        // Should match direct market computation
        let expected = compute_market_order_id(
            1234567890,
            "BTC-USD",
            &owner,
            true,
            0.1,
            false,
        );

        assert_eq!(id, expected);
    }

    #[test]
    fn test_tif_affects_id() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let nonce = 1234567890u64;

        let gtc_id = compute_limit_order_id(
            nonce, "BTC-USD", &owner, true, 0.1, 100000.0, TimeInForce::Gtc, false
        );
        let ioc_id = compute_limit_order_id(
            nonce, "BTC-USD", &owner, true, 0.1, 100000.0, TimeInForce::Ioc, false
        );
        let alo_id = compute_limit_order_id(
            nonce, "BTC-USD", &owner, true, 0.1, 100000.0, TimeInForce::Alo, false
        );

        // Different TIF = different IDs
        assert_ne!(gtc_id, ioc_id);
        assert_ne!(gtc_id, alo_id);
        assert_ne!(ioc_id, alo_id);
    }
}
