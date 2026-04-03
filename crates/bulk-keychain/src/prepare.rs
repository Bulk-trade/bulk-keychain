//! Message preparation for external wallet signing.

use crate::order_id::compute_order_item_id_at_index;
use crate::sdk_compat::serialize_for_sdk_signing;
use crate::types::*;
use crate::{Error, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Threshold for switching to parallel preparation.
const PARALLEL_THRESHOLD: usize = 10;

/// Prepared message for external signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedMessage {
    /// Raw canonical BULK-SDK message bytes to sign.
    #[serde(with = "serde_bytes")]
    pub message_bytes: Vec<u8>,
    /// Optional pre-computed order ID for single order transactions.
    pub order_id: Option<String>,
    /// Optional pre-computed order IDs for multi-order transactions.
    pub order_ids: Option<Vec<String>>,
    /// Compact tagged actions.
    pub actions: Vec<serde_json::Value>,
    /// Account pubkey (base58).
    pub account: String,
    /// Signer pubkey (base58).
    pub signer: String,
    /// Nonce.
    pub nonce: u64,
}

impl PreparedMessage {
    #[inline]
    pub fn message_base58(&self) -> String {
        bs58::encode(&self.message_bytes).into_string()
    }

    #[inline]
    pub fn message_base64(&self) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD.encode(&self.message_bytes)
    }

    #[inline]
    pub fn message_hex(&self) -> String {
        hex::encode(&self.message_bytes)
    }
}

/// Prepare a single order item transaction.
pub fn prepare_message(
    item: OrderItem,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let action = Action::Order { orders: vec![item] };
    prepare_action(&action, account, signer, nonce)
}

/// Prepare an atomic multi-item order transaction.
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

/// Prepare a faucet transaction.
pub fn prepare_faucet(
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let action = Action::Faucet(Faucet::new(*account));
    prepare_action(&action, account, signer, nonce)
}

/// Prepare an agent wallet creation/deletion transaction.
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

/// Prepare a user settings transaction.
pub fn prepare_user_settings(
    settings: UserSettings,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let action = Action::UpdateUserSettings(settings);
    prepare_action(&action, account, signer, nonce)
}

/// Low-level action preparation.
pub fn prepare_action(
    action: &Action,
    account: &Pubkey,
    signer: Option<&Pubkey>,
    nonce: Option<u64>,
) -> Result<PreparedMessage> {
    let signer_pubkey = signer.unwrap_or(account);
    let nonce = nonce.unwrap_or_else(crate::nonce::current_timestamp_millis);

    let mut message_bytes = Vec::with_capacity(512);
    serialize_for_sdk_signing(action, nonce, account, &mut message_bytes)?;

    let actions = action_to_json(action)?;
    let order_id = compute_action_order_id(action, nonce, account);
    let order_ids = compute_action_order_ids(action, nonce, account);

    Ok(PreparedMessage {
        message_bytes,
        order_id,
        order_ids,
        actions,
        account: account.to_base58(),
        signer: signer_pubkey.to_base58(),
        nonce,
    })
}

fn compute_action_order_id(action: &Action, nonce: u64, account: &Pubkey) -> Option<String> {
    match action {
        Action::Order { orders } if orders.len() == 1 => {
            let mut scratch = Vec::with_capacity(96);
            compute_order_item_id_at_index(&orders[0], 0, nonce, account, &mut scratch)
                .map(|id| id.to_base58())
        }
        _ => None,
    }
}

fn compute_action_order_ids(action: &Action, nonce: u64, account: &Pubkey) -> Option<Vec<String>> {
    match action {
        Action::Order { orders } if orders.len() > 1 => {
            let mut scratch = Vec::with_capacity(96);
            let mut ids = Vec::with_capacity(orders.len());
            for (idx, item) in orders.iter().enumerate() {
                if let Some(id) =
                    compute_order_item_id_at_index(item, idx as u32, nonce, account, &mut scratch)
                {
                    ids.push(id.to_base58());
                }
            }
            if ids.is_empty() {
                None
            } else {
                Some(ids)
            }
        }
        _ => None,
    }
}

/// Prepare multiple independent order item transactions.
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
        items
            .into_iter()
            .enumerate()
            .map(|(i, item)| prepare_single_item(item, account, signer_pubkey, base + i as u64))
            .collect()
    } else {
        items
            .into_par_iter()
            .enumerate()
            .map(|(i, item)| prepare_single_item(item, account, signer_pubkey, base + i as u64))
            .collect()
    }
}

fn prepare_single_item(
    item: OrderItem,
    account: &Pubkey,
    signer: &Pubkey,
    nonce: u64,
) -> Result<PreparedMessage> {
    let mut scratch = Vec::with_capacity(96);
    let order_id = compute_order_item_id_at_index(&item, 0, nonce, account, &mut scratch)
        .map(|id| id.to_base58());
    let action = Action::Order { orders: vec![item] };

    let mut message_bytes = Vec::with_capacity(512);
    serialize_for_sdk_signing(&action, nonce, account, &mut message_bytes)?;
    let actions = action_to_json(&action)?;

    Ok(PreparedMessage {
        message_bytes,
        order_id,
        order_ids: None,
        actions,
        account: account.to_base58(),
        signer: signer.to_base58(),
        nonce,
    })
}

/// Finalize a prepared message with a base58 signature.
pub fn finalize_transaction(prepared: PreparedMessage, signature: &str) -> SignedTransaction {
    SignedTransaction {
        actions: prepared.actions,
        nonce: prepared.nonce,
        account: prepared.account,
        signer: prepared.signer,
        signature: signature.to_string(),
        order_id: prepared.order_id,
        order_ids: prepared.order_ids,
    }
}

/// Finalize a prepared message with raw signature bytes.
pub fn finalize_transaction_bytes(
    prepared: PreparedMessage,
    signature: &[u8],
) -> SignedTransaction {
    let signature_b58 = bs58::encode(signature).into_string();
    finalize_transaction(prepared, &signature_b58)
}

/// Finalize many prepared messages with aligned signatures.
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

fn action_to_json(action: &Action) -> Result<Vec<serde_json::Value>> {
    match action {
        Action::Order { orders } => orders.iter().map(order_item_to_json).collect(),
        Action::Faucet(faucet) => {
            let mut faucet_obj = json!({ "u": faucet.user.to_base58() });
            if let Some(amount) = faucet.amount {
                faucet_obj["amount"] = json!(amount);
            }
            Ok(vec![json!({ "faucet": faucet_obj })])
        }
        Action::AgentWalletCreation(agent) => Ok(vec![json!({
            "agentWalletCreation": {
                "a": agent.agent.to_base58(),
                "d": agent.delete
            }
        })]),
        Action::UpdateUserSettings(settings) => {
            let m: serde_json::Map<String, serde_json::Value> = settings
                .max_leverage
                .iter()
                .map(|(symbol, lev)| (symbol.clone(), json!(lev)))
                .collect();
            Ok(vec![json!({ "updateUserSettings": { "m": m } })])
        }
        Action::Oracle { oracles } => Ok(oracles
            .iter()
            .map(|o| {
                json!({
                    "px": {
                        "t": o.timestamp,
                        "c": o.asset,
                        "px": o.price
                    }
                })
            })
            .collect()),
        Action::PythOracle { oracles } => {
            let entries: Vec<_> = oracles
                .iter()
                .map(|o| {
                    json!({
                        "t": o.timestamp,
                        "fi": o.feed_index,
                        "px": o.price,
                        "e": o.exponent
                    })
                })
                .collect();
            Ok(vec![json!({ "o": { "oracles": entries } })])
        }
        Action::WhitelistFaucet(action) => Ok(vec![json!({
            "whitelistFaucet": {
                "target": action.target.to_base58(),
                "whitelist": action.whitelist
            }
        })]),
    }
}

fn order_item_to_json(item: &OrderItem) -> Result<serde_json::Value> {
    match item {
        OrderItem::Order(order) => match &order.order_type {
            OrderType::Limit { tif } => {
                let tif_str = match tif {
                    TimeInForce::Gtc => "GTC",
                    TimeInForce::Ioc => "IOC",
                    TimeInForce::Alo => "ALO",
                };
                Ok(json!({
                    "l": {
                        "c": order.symbol,
                        "b": order.is_buy,
                        "px": order.price,
                        "sz": order.size,
                        "tif": tif_str,
                        "r": order.reduce_only
                    }
                }))
            }
            OrderType::Trigger {
                is_market,
                trigger_px: _,
            } => {
                if !is_market {
                    return Err(Error::InvalidOrder(
                        "trigger orders are not supported by BULK API; use market".to_string(),
                    ));
                }
                Ok(json!({
                    "m": {
                        "c": order.symbol,
                        "b": order.is_buy,
                        "sz": order.size,
                        "r": order.reduce_only
                    }
                }))
            }
        },
        OrderItem::Modify(modify) => Ok(json!({
            "mod": {
                "oid": modify.order_id.to_base58(),
                "c": modify.symbol,
                "sz": modify.amount
            }
        })),
        OrderItem::Cancel(cancel) => Ok(json!({
            "cx": {
                "c": cancel.symbol,
                "oid": cancel.order_id.to_base58()
            }
        })),
        OrderItem::CancelAll(cancel_all) => Ok(json!({
            "cxa": {
                "c": cancel_all.symbols
            }
        })),
        OrderItem::Stop(stop) => Ok(json!({
            "st": {
                "c": stop.symbol,
                "d": stop.is_buy,
                "sz": stop.size,
                "tr": stop.trigger_price,
                "lim": stop.limit_price
            }
        })),
        OrderItem::TakeProfit(tp) => Ok(json!({
            "tp": {
                "c": tp.symbol,
                "d": tp.is_buy,
                "sz": tp.size,
                "tr": tp.trigger_price,
                "lim": tp.limit_price
            }
        })),
        OrderItem::RangeOco(rng) => Ok(json!({
            "rng": {
                "c": rng.symbol,
                "d": rng.is_buy,
                "sz": rng.size,
                "pmin": rng.collar_min,
                "pmax": rng.collar_max,
                "lmin": rng.limit_min,
                "lmax": rng.limit_max
            }
        })),
        OrderItem::TriggerBasket(trig) => {
            let nested: Result<Vec<_>> = trig.actions.iter().map(order_item_to_json).collect();
            Ok(json!({
                "trig": {
                    "c": trig.symbol,
                    "d": trig.is_buy,
                    "tr": trig.trigger_price,
                    "a": nested?
                }
            }))
        }
    }
}

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
        assert_eq!(prepared.nonce, 1234567890);
        assert_eq!(prepared.actions.len(), 1);
        assert!(prepared.actions[0].get("l").is_some());
    }

    #[test]
    fn test_prepare_modify_uses_compact_sdk_keys() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();
        let modify = Modify::new(Hash::random(), "BTC-USD", 0.25);
        let prepared =
            prepare_message(OrderItem::Modify(modify), &account, None, Some(1234567890)).unwrap();

        let mod_obj = prepared.actions[0].get("mod").unwrap();
        assert!(mod_obj.get("c").is_some());
        assert!(mod_obj.get("sz").is_some());
        assert!(mod_obj.get("symbol").is_none());
        assert!(mod_obj.get("amount").is_none());
    }

    #[test]
    fn test_prepare_group() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();
        let items: Vec<OrderItem> = vec![
            Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc).into(),
            Order::limit("BTC-USD", false, 99000.0, 0.1, TimeInForce::Gtc).into(),
        ];
        let prepared = prepare_group(items, &account, None, Some(1234567890)).unwrap();
        assert_eq!(prepared.actions.len(), 2);
        assert!(prepared.order_id.is_none());
        assert_eq!(prepared.order_ids.as_ref().map(Vec::len), Some(2));
    }

    #[test]
    fn test_prepare_all_parallel() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();
        let orders: Vec<OrderItem> = (0..20)
            .map(|i| {
                Order::limit(
                    "BTC-USD",
                    i % 2 == 0,
                    100000.0 + i as f64,
                    0.1,
                    TimeInForce::Gtc,
                )
                .into()
            })
            .collect();

        let prepared = prepare_all(orders, &account, None, Some(1000000)).unwrap();
        assert_eq!(prepared.len(), 20);
        for (i, p) in prepared.iter().enumerate() {
            assert_eq!(p.nonce, 1000000 + i as u64);
        }
    }

    #[test]
    fn test_finalize_transaction() {
        let keypair = Keypair::generate();
        let account = keypair.pubkey();
        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let prepared = prepare_message(order.into(), &account, None, Some(1234567890)).unwrap();
        let signed = finalize_transaction(prepared.clone(), "sig");

        assert_eq!(signed.nonce, prepared.nonce);
        assert_eq!(signed.actions, prepared.actions);
        assert_eq!(signed.signature, "sig");
        assert_eq!(signed.order_ids, prepared.order_ids);
    }
}
