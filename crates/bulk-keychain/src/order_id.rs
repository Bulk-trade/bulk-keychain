//! Order ID computation.
//!
//! Order IDs are SHA256 hashes of canonical wincode bytes for a single
//! order action:
//!
//! `[action_count=1] + [order_action] + [nonce] + [account] + [signer]`

use crate::serialize::WincodeSerializer;
use crate::types::*;

#[inline]
fn serialize_order_item_for_id(
    item: &OrderItem,
    nonce: u64,
    account: &Pubkey,
    signer: &Pubkey,
    serializer: &mut WincodeSerializer,
) -> crate::Result<()> {
    serializer.reset();
    serializer.write_u64(1);
    serializer.write_order_item_action(item)?;
    serializer.write_u64(nonce);
    serializer.write_pubkey(account);
    serializer.write_pubkey(signer);
    Ok(())
}

/// Compute order ID for an order item using explicit account and signer.
///
/// Returns `Some(Hash)` only for `OrderItem::Order`, otherwise `None`.
pub fn compute_order_item_id_with_signer(
    item: &OrderItem,
    nonce: u64,
    account: &Pubkey,
    signer: &Pubkey,
) -> Option<Hash> {
    let mut serializer = WincodeSerializer::new();
    compute_order_item_id_with_serializer(item, nonce, account, signer, &mut serializer)
}

/// Compute order ID for an order item using account and optional signer.
///
/// If `signer` is `None`, `account` is used as signer.
/// Returns `Some(Hash)` only for `OrderItem::Order`, otherwise `None`.
#[inline]
pub fn compute_order_item_id_for_account(
    item: &OrderItem,
    nonce: u64,
    account: &Pubkey,
    signer: Option<&Pubkey>,
) -> Option<Hash> {
    let signer_pubkey = signer.unwrap_or(account);
    compute_order_item_id_with_signer(item, nonce, account, signer_pubkey)
}

/// Compute order ID for an order item, assuming `signer == owner`.
///
/// Returns `Some(Hash)` only for `OrderItem::Order`, otherwise `None`.
pub fn compute_order_item_id(item: &OrderItem, nonce: u64, owner: &Pubkey) -> Option<Hash> {
    compute_order_item_id_with_signer(item, nonce, owner, owner)
}

#[inline]
pub(crate) fn compute_order_item_id_with_serializer(
    item: &OrderItem,
    nonce: u64,
    account: &Pubkey,
    signer: &Pubkey,
    serializer: &mut WincodeSerializer,
) -> Option<Hash> {
    if !matches!(item, OrderItem::Order(_)) {
        return None;
    }

    serialize_order_item_for_id(item, nonce, account, signer, serializer).ok()?;
    Some(Hash::from_wincode_bytes(serializer.as_bytes()))
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
        client_id: None,
    };
    compute_order_id(&order, nonce, owner)
}

/// Compute order ID for an order with explicit account and signer.
#[inline]
pub fn compute_order_id_with_signer(
    order: &Order,
    nonce: u64,
    account: &Pubkey,
    signer: &Pubkey,
) -> Hash {
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
            client_id: order.client_id,
        },
    };

    let item = OrderItem::Order(normalized);
    let mut serializer = WincodeSerializer::new();

    compute_order_item_id_with_serializer(&item, nonce, account, signer, &mut serializer)
        .unwrap_or_else(|| unreachable!("normalized order serialization should always succeed"))
}

/// Compute order ID for an order using account and optional signer.
///
/// If `signer` is `None`, `account` is used as signer.
#[inline]
pub fn compute_order_id_for_account(
    order: &Order,
    nonce: u64,
    account: &Pubkey,
    signer: Option<&Pubkey>,
) -> Hash {
    let signer_pubkey = signer.unwrap_or(account);
    compute_order_id_with_signer(order, nonce, account, signer_pubkey)
}

/// Compute order ID for an order assuming `signer == owner`.
#[inline]
pub fn compute_order_id(order: &Order, nonce: u64, owner: &Pubkey) -> Hash {
    compute_order_id_with_signer(order, nonce, owner, owner)
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
    fn test_compute_order_item_matches_wincode_hash() {
        let owner = Pubkey::from_bytes([11u8; 32]);
        let signer = Pubkey::from_bytes([22u8; 32]);
        let nonce = 987654321u64;

        let item = OrderItem::Order(Order::limit(
            "ETH-USD",
            false,
            3200.5,
            1.25,
            TimeInForce::Ioc,
        ));

        let id = compute_order_item_id_with_signer(&item, nonce, &owner, &signer).unwrap();

        let mut serializer = WincodeSerializer::new();
        serializer.write_u64(1);
        serializer.write_order_item_action(&item).unwrap();
        serializer.write_u64(nonce);
        serializer.write_pubkey(&owner);
        serializer.write_pubkey(&signer);
        let expected = Hash::from_wincode_bytes(serializer.as_bytes());

        assert_eq!(id, expected);
    }

    #[test]
    fn test_signer_affects_order_id() {
        let owner = Pubkey::from_bytes([1u8; 32]);
        let signer_a = Pubkey::from_bytes([2u8; 32]);
        let signer_b = Pubkey::from_bytes([3u8; 32]);
        let order = Order::market("SOL-USD", true, 10.0);

        let id_a = compute_order_id_with_signer(&order, 42, &owner, &signer_a);
        let id_b = compute_order_id_with_signer(&order, 42, &owner, &signer_b);

        assert_ne!(id_a, id_b);
    }

    #[test]
    fn test_compute_order_item_id_for_account_uses_account_as_default_signer() {
        let owner = Pubkey::from_bytes([9u8; 32]);
        let item = OrderItem::Order(Order::limit(
            "BTC-USD",
            true,
            100000.0,
            0.1,
            TimeInForce::Gtc,
        ));

        let direct = compute_order_item_id_with_signer(&item, 123, &owner, &owner);
        let optional = compute_order_item_id_for_account(&item, 123, &owner, None);

        assert_eq!(optional, direct);
    }

    #[test]
    fn test_compute_order_id_for_account_uses_account_as_default_signer() {
        let owner = Pubkey::from_bytes([7u8; 32]);
        let order = Order::market("ETH-USD", false, 2.0);

        let direct = compute_order_id_with_signer(&order, 77, &owner, &owner);
        let optional = compute_order_id_for_account(&order, 77, &owner, None);

        assert_eq!(optional, direct);
    }
}
