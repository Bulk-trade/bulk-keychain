//! Message preparation for external wallet signing
//!
//! This module provides a two-step signing flow for use with external wallets
//! (like Phantom, Privy, etc.) where you don't have access to the private key.
//!
//! ## Flow
//!
//! 1. **Prepare**: Serialize the transaction and get the message bytes
//! 2. **Sign externally**: Pass bytes to wallet's `signMessage()` function
//! 3. **Finalize**: Combine the prepared message with the signature
//!
//! ## Example
//!
//! ```rust,ignore
//! use bulk_keychain::{prepare_message, finalize_transaction, Order, TimeInForce, Pubkey};
//!
//! // Step 1: Prepare the message
//! let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
//! let account = Pubkey::from_base58("user-pubkey")?;
//! let prepared = prepare_message(order.into(), &account, None, None)?;
//!
//! // Step 2: Get signature from external wallet
//! // let signature = wallet.signMessage(prepared.message_bytes);
//!
//! // Step 3: Finalize
//! let signed = finalize_transaction(prepared, "signature-base58");
//! ```

use crate::order_id::compute_order_item_id;
use crate::serialize::WincodeSerializer;
use crate::types::*;
use crate::{Error, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Threshold for switching to parallel preparation
const PARALLEL_THRESHOLD: usize = 10;

/// A prepared message ready for external signing
///
/// Contains all the data needed to sign with an external wallet
/// and then finalize into a complete `SignedTransaction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedMessage {
    /// Raw message bytes to sign (wincode serialized)
    /// Pass this to your wallet's signMessage() function
    #[serde(with = "serde_bytes")]
    pub message_bytes: Vec<u8>,

    /// Pre-computed order/transaction ID (base58)
    /// This matches BULK's server-side ID: SHA256(message_bytes)
    pub order_id: String,

    /// Action JSON for the API request
    pub action: serde_json::Value,

    /// Account public key (base58) - the trading account
    pub account: String,

    /// Signer public key (base58) - who signs (may differ for agent wallets)
    pub signer: String,

    /// Nonce used for this transaction
    pub nonce: u64,
}

impl PreparedMessage {
    /// Get message bytes as base58 string
    #[inline]
    pub fn message_base58(&self) -> String {
        bs58::encode(&self.message_bytes).into_string()
    }

    /// Get message bytes as base64 string
    #[inline]
    pub fn message_base64(&self) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD.encode(&self.message_bytes)
    }

    /// Get message bytes as hex string
    #[inline]
    pub fn message_hex(&self) -> String {
        hex::encode(&self.message_bytes)
    }
}

// ============================================================================
// Single message preparation
// ============================================================================

/// Prepare a single order item for external signing
///
/// This is the main entry point for preparing messages. It serializes the
/// transaction and returns everything needed to sign with an external wallet.
///
/// # Arguments
///
/// * `item` - The order item (order, cancel, or cancelAll)
/// * `account` - The trading account public key
/// * `signer` - The signing wallet (defaults to account if None)
/// * `nonce` - Transaction nonce (defaults to current timestamp if None)
///
/// # Example
///
/// ```rust,ignore
/// let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
/// let prepared = prepare_message(order.into(), &account, None, None)?;
/// ```
pub fn prepare_message(
    item: OrderItem,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let action = Action::Order { orders: vec![item] };
    prepare_action(&action, account, signer, nonce)
}

/// Prepare a group of orders as a single atomic transaction
///
/// All orders will be in one transaction - they succeed or fail together.
/// Use this for bracket orders (entry + stop loss + take profit).
///
/// # Example
///
/// ```rust,ignore
/// let bracket = vec![entry.into(), stop_loss.into(), take_profit.into()];
/// let prepared = prepare_group(bracket, &account, None, None)?;
/// ```
pub fn prepare_group(
    items: Vec<OrderItem>,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    if items.is_empty() {
        return Err(Error::EmptyOrders);
    }
    let action = Action::Order { orders: items };
    prepare_action(&action, account, signer, nonce)
}

/// Prepare a faucet request for external signing
pub fn prepare_faucet(
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let action = Action::Faucet(Faucet::new(*account));
    prepare_action(&action, account, signer, nonce)
}

/// Prepare agent wallet creation/deletion for external signing
pub fn prepare_agent_wallet(
    agent: &Pubkey,
    delete: bool,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let action = Action::AgentWalletCreation(AgentWallet {
        agent: *agent,
        delete,
    });
    prepare_action(&action, account, signer, nonce)
}

/// Prepare user settings update for external signing
pub fn prepare_user_settings(
    settings: UserSettings,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let action = Action::UpdateUserSettings(settings);
    prepare_action(&action, account, signer, nonce)
}

/// Low-level: Prepare any action for external signing
pub fn prepare_action(
    action: &Action,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let signer_pubkey = signer.unwrap_or(account);
    let nonce = nonce.unwrap_or_else(crate::nonce::current_timestamp_millis);

    // Serialize
    let mut serializer = WincodeSerializer::new();
    serializer.serialize_for_signing(action, nonce, account, signer_pubkey);
    let message_bytes = serializer.into_bytes();

    // Compute order ID using new fixed-point algorithm
    let order_id = compute_action_order_id(action, nonce, account);

    // Build action JSON
    let action_json = action_to_json(action, nonce);

    Ok(PreparedMessage {
        message_bytes,
        order_id,
        action: action_json,
        account: account.to_base58(),
        signer: signer_pubkey.to_base58(),
        nonce,
    })
}

/// Compute order ID for an action using the new fixed-point algorithm
fn compute_action_order_id(action: &Action, nonce: u64, account: &Pubkey) -> String {
    match action {
        Action::Order { orders } if orders.len() == 1 => {
            // Single order: compute OID from the order itself
            compute_order_item_id(&orders[0], nonce, account)
                .map(|h| h.to_base58())
                .unwrap_or_default()
        }
        _ => String::new(), // Multi-order groups, Cancel, CancelAll, Faucet, etc. don't have OIDs
    }
}

// ============================================================================
// Batch message preparation (parallel)
// ============================================================================

/// Prepare multiple orders - each becomes its own transaction
///
/// Each order gets independent confirmation/rejection.
/// Automatically parallelizes when > 10 orders.
///
/// # Example
///
/// ```rust,ignore
/// let orders = vec![order1.into(), order2.into(), order3.into()];
/// let prepared = prepare_all(orders, &account, None, None)?;
/// // Returns Vec<PreparedMessage>, one per order
/// ```
pub fn prepare_all(
    items: Vec<OrderItem>,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    base_nonce: Option<u64>,
) -> Result<Vec<PreparedMessage>> {
    if items.is_empty() {
        return Ok(vec![]);
    }

    let base = base_nonce.unwrap_or_else(crate::nonce::current_timestamp_millis);
    let signer_pubkey = signer.unwrap_or(account);

    if items.len() < PARALLEL_THRESHOLD {
        // Sequential for small batches
        items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                prepare_single_item(item, account, signer_pubkey, base + i as u64)
            })
            .collect()
    } else {
        // Parallel for large batches
        items
            .into_par_iter()
            .enumerate()
            .map(|(i, item)| {
                prepare_single_item(item, account, signer_pubkey, base + i as u64)
            })
            .collect()
    }
}

/// Internal: Prepare a single item (for parallel use)
fn prepare_single_item(
    item: OrderItem,
    account: &Pubkey,
    signer: &Pubkey,
    nonce: u64,
) -> Result<PreparedMessage> {
    // Compute order ID using new fixed-point algorithm
    let order_id = compute_order_item_id(&item, nonce, account)
        .map(|h| h.to_base58())
        .unwrap_or_default();

    let action = Action::Order { orders: vec![item] };

    let mut serializer = WincodeSerializer::new();
    serializer.serialize_for_signing(&action, nonce, account, signer);
    let message_bytes = serializer.into_bytes();

    let action_json = action_to_json(&action, nonce);

    Ok(PreparedMessage {
        message_bytes,
        order_id,
        action: action_json,
        account: account.to_base58(),
        signer: signer.to_base58(),
        nonce,
    })
}

// ============================================================================
// Finalization
// ============================================================================

/// Finalize a prepared message with an external signature
///
/// After your wallet signs the `message_bytes`, call this to create
/// the complete `SignedTransaction` ready for API submission.
///
/// # Arguments
///
/// * `prepared` - The prepared message from `prepare_*` functions
/// * `signature` - The signature from your wallet (base58 encoded)
///
/// # Example
///
/// ```rust,ignore
/// let signature = wallet.signMessage(prepared.message_bytes);
/// let signed = finalize_transaction(prepared, &signature);
/// ```
pub fn finalize_transaction(prepared: PreparedMessage, signature: &str) -> SignedTransaction {
    SignedTransaction {
        action: prepared.action,
        account: prepared.account,
        signer: prepared.signer,
        signature: signature.to_string(),
        order_id: Some(prepared.order_id),
    }
}

/// Finalize with signature bytes (will be base58 encoded)
pub fn finalize_transaction_bytes(prepared: PreparedMessage, signature: &[u8]) -> SignedTransaction {
    let signature_b58 = bs58::encode(signature).into_string();
    finalize_transaction(prepared, &signature_b58)
}

/// Finalize multiple prepared messages with their signatures
///
/// Signatures must be in the same order as the prepared messages.
pub fn finalize_all(
    prepared: Vec<PreparedMessage>,
    signatures: Vec<&str>,
) -> Result<Vec<SignedTransaction>> {
    if prepared.len() != signatures.len() {
        return Err(Error::SignatureMismatch {
            expected: prepared.len(),
            got: signatures.len(),
        });
    }

    Ok(prepared
        .into_iter()
        .zip(signatures)
        .map(|(p, sig)| finalize_transaction(p, sig))
        .collect())
}

// ============================================================================
// JSON conversion (duplicated from sign.rs to avoid coupling)
// ============================================================================

fn action_to_json(action: &Action, nonce: u64) -> serde_json::Value {
    match action {
        Action::Order { orders } => {
            let orders_json: Vec<_> = orders.iter().map(order_item_to_json).collect();
            json!({
                "type": "order",
                "orders": orders_json,
                "nonce": nonce
            })
        }
        Action::Faucet(faucet) => {
            let mut faucet_obj = json!({
                "u": faucet.user.to_base58()
            });
            if let Some(amount) = faucet.amount {
                faucet_obj["amount"] = json!(amount);
            }
            json!({
                "type": "faucet",
                "faucet": faucet_obj,
                "nonce": nonce
            })
        }
        Action::AgentWalletCreation(agent) => {
            json!({
                "type": "agentWalletCreation",
                "agent": {
                    "a": agent.agent.to_base58(),
                    "d": agent.delete
                },
                "nonce": nonce
            })
        }
        Action::UpdateUserSettings(settings) => {
            let leverage_arr: Vec<_> = settings
                .max_leverage
                .iter()
                .map(|(symbol, lev)| json!([symbol, lev]))
                .collect();
            json!({
                "type": "updateUserSettings",
                "settings": {
                    "m": leverage_arr
                },
                "nonce": nonce
            })
        }
        Action::Oracle { oracles } => {
            let oracles_json: Vec<_> = oracles
                .iter()
                .map(|o| {
                    json!({
                        "t": o.timestamp,
                        "c": o.asset,
                        "px": o.price
                    })
                })
                .collect();
            json!({
                "type": "oracle",
                "oracles": oracles_json,
                "nonce": nonce
            })
        }
    }
}

fn order_item_to_json(item: &OrderItem) -> serde_json::Value {
    match item {
        OrderItem::Order(order) => {
            let mut order_obj = json!({
                "c": order.symbol,
                "b": order.is_buy,
                "px": order.price,
                "sz": order.size,
                "r": order.reduce_only,
                "t": order_type_to_json(&order.order_type)
            });
            if let Some(ref cloid) = order.client_id {
                order_obj["cloid"] = json!(cloid.to_base58());
            }
            json!({ "order": order_obj })
        }
        OrderItem::Cancel(cancel) => {
            json!({
                "cancel": {
                    "c": cancel.symbol,
                    "oid": cancel.order_id.to_base58()
                }
            })
        }
        OrderItem::CancelAll(cancel_all) => {
            json!({
                "cancelAll": {
                    "c": cancel_all.symbols
                }
            })
        }
    }
}

fn order_type_to_json(order_type: &OrderType) -> serde_json::Value {
    match order_type {
        OrderType::Limit { tif } => {
            let tif_str = match tif {
                TimeInForce::Gtc => "GTC",
                TimeInForce::Ioc => "IOC",
                TimeInForce::Alo => "ALO",
            };
            json!({ "limit": { "tif": tif_str } })
        }
        OrderType::Trigger {
            is_market,
            trigger_px,
        } => {
            json!({
                "trigger": {
                    "is_market": is_market,
                    "triggerPx": trigger_px
                }
            })
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Keypair;

    #[test]
    fn test_prepare_message() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let prepared = prepare_message(order.into(), &account, None, Some(1234567890)).unwrap();

        assert!(!prepared.message_bytes.is_empty());
        assert!(!prepared.order_id.is_empty());
        assert_eq!(prepared.account, account.to_base58());
        assert_eq!(prepared.signer, account.to_base58()); // defaults to account
        assert_eq!(prepared.nonce, 1234567890);
    }

    #[test]
    fn test_prepare_with_different_signer() {
        let account_kp = Keypair::generate();
        let signer_kp = Keypair::generate();
        let account = account_kp.pubkey();
        let signer = signer_kp.pubkey();

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let prepared = prepare_message(order.into(), &account, Some(&signer), Some(1234567890)).unwrap();

        assert_eq!(prepared.account, account.to_base58());
        assert_eq!(prepared.signer, signer.to_base58());
        assert_ne!(prepared.account, prepared.signer);
    }

    #[test]
    fn test_prepare_all_parallel() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();

        let orders: Vec<OrderItem> = (0..20)
            .map(|i| Order::limit("BTC-USD", i % 2 == 0, 100000.0 + i as f64, 0.1, TimeInForce::Gtc).into())
            .collect();

        let prepared = prepare_all(orders, &account, None, Some(1000000)).unwrap();
        assert_eq!(prepared.len(), 20);

        // Each should have a unique nonce and order ID
        for (i, p) in prepared.iter().enumerate() {
            assert_eq!(p.nonce, 1000000 + i as u64);
        }

        // Order IDs should all be unique
        let order_ids: std::collections::HashSet<_> = prepared.iter().map(|p| &p.order_id).collect();
        assert_eq!(order_ids.len(), 20);
    }

    #[test]
    fn test_finalize_transaction() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let prepared = prepare_message(order.into(), &account, None, Some(1234567890)).unwrap();

        // Simulate wallet signing (in practice, wallet would sign prepared.message_bytes)
        let fake_signature = "FakeSignatureBase58EncodedString123456789";
        let signed = finalize_transaction(prepared.clone(), fake_signature);

        assert_eq!(signed.signature, fake_signature);
        assert_eq!(signed.account, prepared.account);
        assert_eq!(signed.signer, prepared.signer);
        assert_eq!(signed.order_id, Some(prepared.order_id));
    }

    #[test]
    fn test_message_format_helpers() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let prepared = prepare_message(order.into(), &account, None, Some(1234567890)).unwrap();

        // All format helpers should work
        let b58 = prepared.message_base58();
        let b64 = prepared.message_base64();
        let hex = prepared.message_hex();

        assert!(!b58.is_empty());
        assert!(!b64.is_empty());
        assert!(!hex.is_empty());

        // Should be different representations
        assert_ne!(b58, b64);
        assert_ne!(b58, hex);
    }

    #[test]
    fn test_prepare_group() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();

        let bracket: Vec<OrderItem> = vec![
            Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc).into(),
            Order::limit("BTC-USD", false, 99000.0, 0.1, TimeInForce::Gtc).into(),
            Order::limit("BTC-USD", false, 110000.0, 0.1, TimeInForce::Gtc).into(),
        ];

        let prepared = prepare_group(bracket, &account, None, Some(1234567890)).unwrap();

        // All 3 orders should be in one transaction
        assert_eq!(prepared.action["orders"].as_array().unwrap().len(), 3);
    }
}
