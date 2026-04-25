//! Order ID computation.
//!
//! Order IDs are SHA256 hashes of canonical BULK-SDK action bytes:
//!
//! `[seqno] + [bincode(action)] + [account] + [nonce]`
//!
//! Where:
//! - `seqno` is the action index in the transaction (u32 LE)
//! - `action` uses BULK-SDK's bincode/serde representation
//! - `account` is the trading account pubkey bytes
//! - `nonce` is u64 LE

use crate::sdk_compat::compute_order_item_id_with_seqno;
use crate::types::*;

/// Compute order ID for an order item, assuming `signer == owner`.
///
/// Returns `Some(Hash)` only for `OrderItem::Order`, otherwise `None`.
pub fn compute_order_item_id(item: &OrderItem, nonce: u64, owner: &Pubkey) -> Option<Hash> {
    let mut scratch = Vec::with_capacity(96);
    compute_order_item_id_at_index(item, 0, nonce, owner, &mut scratch)
}

#[inline]
pub(crate) fn compute_order_item_id_at_index(
    item: &OrderItem,
    seqno: u32,
    nonce: u64,
    account: &Pubkey,
    scratch: &mut Vec<u8>,
) -> Option<Hash> {
    compute_order_item_id_with_seqno(item, seqno, nonce, account, scratch)
}

/// Compute order ID for a limit order.
#[inline]
#[allow(clippy::too_many_arguments)]
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
    let order = Order {
        symbol: market.to_string(),
        is_buy,
        price,
        size: amount,
        order_type: OrderType::Limit { tif },
        reduce_only,
        iso: false,
        client_id: None,
    };
    compute_order_id(&order, nonce, owner)
}

/// Compute order ID for a market order.
#[inline]
pub fn compute_market_order_id(
    nonce: u64,
    market: &str,
    owner: &Pubkey,
    is_buy: bool,
    amount: f64,
    reduce_only: bool,
) -> Hash {
    let order = Order {
        symbol: market.to_string(),
        is_buy,
        price: 0.0,
        size: amount,
        order_type: OrderType::Trigger {
            is_market: true,
            trigger_px: 0.0,
        },
        reduce_only,
        iso: false,
        client_id: None,
    };
    compute_order_id(&order, nonce, owner)
}

#[inline]
fn compute_order_id_at_index(order: &Order, seqno: u32, nonce: u64, owner: &Pubkey) -> Hash {
    let normalized = match &order.order_type {
        OrderType::Limit { .. } => order.clone(),
        // Trigger orders in this API are represented as market orders.
        OrderType::Trigger { .. } => Order {
            symbol: order.symbol.clone(),
            is_buy: order.is_buy,
            price: order.price,
            size: order.size,
            order_type: OrderType::Trigger {
                is_market: true,
                trigger_px: 0.0,
            },
            reduce_only: order.reduce_only,
            iso: order.iso,
            client_id: order.client_id,
        },
    };

    let item = OrderItem::Order(normalized);
    let mut scratch = Vec::with_capacity(96);
    compute_order_item_id_with_seqno(&item, seqno, nonce, owner, &mut scratch)
        .unwrap_or_else(|| unreachable!("normalized order serialization should always succeed"))
}

/// Compute order ID for an order assuming `signer == owner`.
#[inline]
pub fn compute_order_id(order: &Order, nonce: u64, owner: &Pubkey) -> Hash {
    compute_order_id_at_index(order, 0, nonce, owner)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_order_id_deterministic() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let nonce = 1234567890u64;

        let id1 = compute_limit_order_id(
            nonce,
            "BTC-USD",
            &owner,
            true,
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

        let market_id = compute_market_order_id(nonce, "BTC-USD", &owner, true, 0.1, false);

        assert_ne!(limit_id, market_id);
    }

    #[test]
    fn test_tif_affects_id() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let nonce = 1234567890u64;

        let gtc_id = compute_limit_order_id(
            nonce,
            "BTC-USD",
            &owner,
            true,
            0.1,
            100000.0,
            TimeInForce::Gtc,
            false,
        );

        let ioc_id = compute_limit_order_id(
            nonce,
            "BTC-USD",
            &owner,
            true,
            0.1,
            100000.0,
            TimeInForce::Ioc,
            false,
        );

        let alo_id = compute_limit_order_id(
            nonce,
            "BTC-USD",
            &owner,
            true,
            0.1,
            100000.0,
            TimeInForce::Alo,
            false,
        );

        assert_ne!(gtc_id, ioc_id);
        assert_ne!(ioc_id, alo_id);
        assert_ne!(gtc_id, alo_id);
    }

    #[test]
    fn test_compute_order_id_limit() {
        let owner = Pubkey::from_bytes([42u8; 32]);
        let order = Order::limit("BTC-USD", true, 50000.0, 0.5, TimeInForce::Gtc);

        let id = compute_order_id(&order, 1234567890, &owner);

        let expected = compute_limit_order_id(
            1234567890,
            "BTC-USD",
            &owner,
            true,
            0.5,
            50000.0,
            TimeInForce::Gtc,
            false,
        );

        assert_eq!(id, expected);
    }

    #[test]
    fn test_compute_order_id_market() {
        let owner = Pubkey::from_bytes([42u8; 32]);
        let order = Order::market("BTC-USD", true, 0.1);

        let id = compute_order_id(&order, 1234567890, &owner);

        let expected = compute_market_order_id(1234567890, "BTC-USD", &owner, true, 0.1, false);

        assert_eq!(id, expected);
    }

    #[test]
    fn test_compute_order_item_id_only_for_order_items() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let cancel = OrderItem::Cancel(Cancel::new("BTC-USD", Hash::random()));

        assert!(compute_order_item_id(&cancel, 123, &owner).is_none());
    }

    #[test]
    fn test_compute_order_item_matches_bulk_sdk_vector() {
        let owner = Pubkey::from_base58("7DHvrCZMMLZ2ovNfKaGpvJZXAQyydbTz6dM7w7qXtzX5").unwrap();
        let nonce = 1772814622879766u64;

        let item = OrderItem::Order(Order::limit(
            "BTC-USD",
            true,
            68495.0,
            1.9402,
            TimeInForce::Gtc,
        ));

        let id = compute_order_item_id(&item, nonce, &owner).unwrap();
        assert_eq!(
            id.to_base58(),
            "9rsNanYXKgkHaB12DJMW85cLh6dTGkZ53jv1shnaTZ8J"
        );
    }

    #[test]
    fn test_compute_order_id_matches_same_owner_path() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let order = Order::market("SOL-USD", true, 10.0);

        let id_a = compute_order_id(&order, 42, &owner);
        let id_b = compute_order_id_at_index(&order, 0, 42, &owner);

        assert_eq!(id_a, id_b);
    }
}
