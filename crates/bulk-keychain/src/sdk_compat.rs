//! Canonical BULK-SDK-compatible serialization.

use crate::types::*;
use crate::{Error, Result};
use serde::Serialize;
use serde::Serializer;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const SCALE: f64 = 1e8;

mod serde_hash {
    use super::*;

    pub fn serialize<S: Serializer>(
        val: &Hash,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&val.to_base58())
        } else {
            serializer.serialize_bytes(val.as_bytes())
        }
    }
}

mod serde_pubkey {
    use super::*;

    pub fn serialize<S: Serializer>(
        val: &Pubkey,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&val.to_base58())
        } else {
            serializer.serialize_bytes(val.as_bytes())
        }
    }
}

mod serde_safe_f64 {
    use super::*;

    pub fn serialize<S: Serializer>(
        val: &f64,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&val.to_string())
        } else {
            let fixed = (val * SCALE).round() as u64;
            serializer.serialize_u64(fixed)
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum TxTimeInForce {
    Gtc,
    Ioc,
    Alo,
}

impl From<TimeInForce> for TxTimeInForce {
    #[inline]
    fn from(value: TimeInForce) -> Self {
        match value {
            TimeInForce::Gtc => Self::Gtc,
            TimeInForce::Ioc => Self::Ioc,
            TimeInForce::Alo => Self::Alo,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct TxMarketOrder {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "b")]
    is_buy: bool,
    #[serde(rename = "sz", with = "serde_safe_f64")]
    size: f64,
    #[serde(rename = "r")]
    reduce_only: bool,
}

#[derive(Clone, Debug, Serialize)]
struct TxLimitOrder {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "b")]
    is_buy: bool,
    #[serde(rename = "px", with = "serde_safe_f64")]
    price: f64,
    #[serde(rename = "sz", with = "serde_safe_f64")]
    size: f64,
    #[serde(rename = "tif")]
    tif: TxTimeInForce,
    #[serde(rename = "r")]
    reduce_only: bool,
}

#[derive(Clone, Debug, Serialize)]
struct TxModifyOrder {
    #[serde(with = "serde_hash", rename = "oid")]
    order_id: Hash,
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "sz")]
    amount: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TxCancelOrder {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(with = "serde_hash", rename = "oid")]
    oid: Hash,
}

#[derive(Clone, Debug, Serialize)]
struct TxCancelAll {
    #[serde(rename = "c")]
    symbols: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct TxPrice {
    #[serde(rename = "t")]
    timestamp: u64,
    #[serde(rename = "c")]
    asset: String,
    #[serde(rename = "px")]
    price: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TxPythPrice {
    #[serde(rename = "t")]
    timestamp: u64,
    #[serde(rename = "fi")]
    id: u64,
    #[serde(rename = "px")]
    px: u64,
    #[serde(rename = "e")]
    exponent: i16,
}

#[derive(Clone, Debug, Serialize)]
struct TxPythOracle {
    oracles: Vec<TxPythPrice>,
}

#[derive(Clone, Debug, Serialize)]
struct TxStop {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "d")]
    is_buy: bool,
    #[serde(rename = "sz", with = "serde_safe_f64")]
    size: f64,
    #[serde(rename = "tr", with = "serde_safe_f64")]
    trigger_price: f64,
    #[serde(rename = "lim", with = "serde_safe_f64")]
    limit_price: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TxTakeProfit {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "d")]
    is_buy: bool,
    #[serde(rename = "sz", with = "serde_safe_f64")]
    size: f64,
    #[serde(rename = "tr", with = "serde_safe_f64")]
    trigger_price: f64,
    #[serde(rename = "lim", with = "serde_safe_f64")]
    limit_price: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TxRangeOco {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "d")]
    is_buy: bool,
    #[serde(rename = "sz", with = "serde_safe_f64")]
    size: f64,
    #[serde(rename = "pmin", with = "serde_safe_f64")]
    collar_min: f64,
    #[serde(rename = "pmax", with = "serde_safe_f64")]
    collar_max: f64,
    #[serde(rename = "lmin", with = "serde_safe_f64")]
    limit_min: f64,
    #[serde(rename = "lmax", with = "serde_safe_f64")]
    limit_max: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TxTriggerBasket {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "d")]
    is_buy: bool,
    #[serde(rename = "tr", with = "serde_safe_f64")]
    trigger_price: f64,
    #[serde(rename = "actions")]
    actions: Vec<TxAction>,
}

#[derive(Clone, Debug, Serialize)]
struct TxOnFill {
    #[serde(rename = "p")]
    parent_seqno: u32,
    #[serde(rename = "actions")]
    actions: Vec<TxAction>,
}

#[derive(Clone, Debug, Serialize)]
struct TxTrailingStop {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "b")]
    is_buy: bool,
    #[serde(rename = "sz", with = "serde_safe_f64")]
    size: f64,
    #[serde(rename = "trb")]
    trail_bps: u32,
    #[serde(rename = "stb")]
    step_bps: u32,
    #[serde(rename = "lim")]
    limit_price: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct TxFaucet {
    #[serde(with = "serde_pubkey", rename = "u")]
    user: Pubkey,
    amount: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct TxAgentWalletCreation {
    #[serde(with = "serde_pubkey", rename = "a")]
    agent: Pubkey,
    #[serde(rename = "d")]
    delete: bool,
}

#[derive(Clone, Debug, Serialize)]
struct TxUpdateUserSettings {
    #[serde(rename = "m")]
    max_leverage: HashMap<String, f64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TxWhitelistFaucet {
    #[serde(with = "serde_pubkey")]
    target: Pubkey,
    whitelist: bool,
}

#[derive(Clone, Debug, Serialize)]
enum TxAction {
    #[serde(rename = "m")]
    MarketOrder(TxMarketOrder),
    #[serde(rename = "l")]
    LimitOrder(TxLimitOrder),
    #[serde(rename = "mod")]
    ModifyOrder(TxModifyOrder),
    #[serde(rename = "cx")]
    Cancel(TxCancelOrder),
    #[serde(rename = "cxa")]
    CancelAll(TxCancelAll),
    #[serde(rename = "st")]
    Stop(TxStop),
    #[serde(rename = "tp")]
    TakeProfit(TxTakeProfit),
    #[serde(rename = "rng")]
    RangeOco(TxRangeOco),
    #[serde(rename = "trig")]
    TriggerBasket(TxTriggerBasket),
    #[serde(rename = "of")]
    OnFill(TxOnFill),
    #[serde(rename = "trl")]
    TrailingStop(TxTrailingStop),
    #[serde(rename = "px")]
    Price(TxPrice),
    #[serde(rename = "o")]
    PythOracle(TxPythOracle),
    Faucet(TxFaucet),
    AgentWalletCreation(TxAgentWalletCreation),
    UpdateUserSettings(TxUpdateUserSettings),
    WhitelistFaucet(TxWhitelistFaucet),
}

#[inline]
fn order_item_to_tx_action(item: &OrderItem) -> Result<TxAction> {
    match item {
        OrderItem::Order(order) => match order.order_type {
            OrderType::Limit { tif } => Ok(TxAction::LimitOrder(TxLimitOrder {
                symbol: order.symbol.clone(),
                is_buy: order.is_buy,
                price: order.price,
                size: order.size,
                tif: TxTimeInForce::from(tif),
                reduce_only: order.reduce_only,
            })),
            OrderType::Trigger {
                is_market,
                trigger_px: _,
            } => {
                if !is_market {
                    return Err(Error::InvalidOrder(
                        "trigger orders are not supported by BULK API; use market".to_string(),
                    ));
                }
                Ok(TxAction::MarketOrder(TxMarketOrder {
                    symbol: order.symbol.clone(),
                    is_buy: order.is_buy,
                    size: order.size,
                    reduce_only: order.reduce_only,
                }))
            }
        },
        OrderItem::Modify(modify) => Ok(TxAction::ModifyOrder(TxModifyOrder {
            order_id: modify.order_id,
            symbol: modify.symbol.clone(),
            amount: modify.amount,
        })),
        OrderItem::Cancel(cancel) => Ok(TxAction::Cancel(TxCancelOrder {
            symbol: cancel.symbol.clone(),
            oid: cancel.order_id,
        })),
        OrderItem::CancelAll(cancel_all) => Ok(TxAction::CancelAll(TxCancelAll {
            symbols: cancel_all.symbols.clone(),
        })),
        OrderItem::Stop(stop) => Ok(TxAction::Stop(TxStop {
            symbol: stop.symbol.clone(),
            is_buy: stop.is_buy,
            size: stop.size,
            trigger_price: stop.trigger_price,
            limit_price: stop.limit_price,
        })),
        OrderItem::TakeProfit(tp) => Ok(TxAction::TakeProfit(TxTakeProfit {
            symbol: tp.symbol.clone(),
            is_buy: tp.is_buy,
            size: tp.size,
            trigger_price: tp.trigger_price,
            limit_price: tp.limit_price,
        })),
        OrderItem::RangeOco(rng) => Ok(TxAction::RangeOco(TxRangeOco {
            symbol: rng.symbol.clone(),
            is_buy: rng.is_buy,
            size: rng.size,
            collar_min: rng.collar_min,
            collar_max: rng.collar_max,
            limit_min: rng.limit_min,
            limit_max: rng.limit_max,
        })),
        OrderItem::TriggerBasket(trig) => {
            let actions: Result<Vec<TxAction>> =
                trig.actions.iter().map(order_item_to_tx_action).collect();
            Ok(TxAction::TriggerBasket(TxTriggerBasket {
                symbol: trig.symbol.clone(),
                is_buy: trig.is_buy,
                trigger_price: trig.trigger_price,
                actions: actions?,
            }))
        }
        OrderItem::OnFill(of) => {
            let actions: Result<Vec<TxAction>> =
                of.actions.iter().map(order_item_to_tx_action).collect();
            Ok(TxAction::OnFill(TxOnFill {
                parent_seqno: of.p,
                actions: actions?,
            }))
        }
        OrderItem::TrailingStop(trl) => Ok(TxAction::TrailingStop(TxTrailingStop {
            symbol: trl.symbol.clone(),
            is_buy: trl.is_buy,
            size: trl.size,
            trail_bps: trl.trail_bps,
            step_bps: trl.step_bps,
            limit_price: trl.limit_price,
        })),
    }
}

#[inline]
fn action_to_tx_actions(action: &Action) -> Result<Vec<TxAction>> {
    match action {
        Action::Order { orders } => orders.iter().map(order_item_to_tx_action).collect(),
        Action::Oracle { oracles } => Ok(oracles
            .iter()
            .map(|oracle| {
                TxAction::Price(TxPrice {
                    timestamp: oracle.timestamp,
                    asset: oracle.asset.clone(),
                    price: oracle.price,
                })
            })
            .collect()),
        Action::PythOracle { oracles } => Ok(vec![TxAction::PythOracle(TxPythOracle {
            oracles: oracles
                .iter()
                .map(|oracle| TxPythPrice {
                    timestamp: oracle.timestamp,
                    id: oracle.feed_index,
                    px: oracle.price,
                    exponent: oracle.exponent,
                })
                .collect(),
        })]),
        Action::Faucet(faucet) => Ok(vec![TxAction::Faucet(TxFaucet {
            user: faucet.user,
            amount: faucet.amount,
        })]),
        Action::AgentWalletCreation(agent) => {
            Ok(vec![TxAction::AgentWalletCreation(TxAgentWalletCreation {
                agent: agent.agent,
                delete: agent.delete,
            })])
        }
        Action::UpdateUserSettings(settings) => {
            let mut max_leverage = HashMap::with_capacity(settings.max_leverage.len());
            for (symbol, leverage) in &settings.max_leverage {
                max_leverage.insert(symbol.clone(), *leverage);
            }
            Ok(vec![TxAction::UpdateUserSettings(TxUpdateUserSettings {
                max_leverage,
            })])
        }
        Action::WhitelistFaucet(action) => Ok(vec![TxAction::WhitelistFaucet(TxWhitelistFaucet {
            target: action.target,
            whitelist: action.whitelist,
        })]),
    }
}

#[inline]
fn serialize_into_buffer<T: Serialize>(value: &T, buffer: &mut Vec<u8>) -> Result<()> {
    buffer.clear();
    bincode::serialize_into(&mut *buffer, value)
        .map_err(|e| Error::SerializationError(e.to_string()))
}

#[inline]
pub(crate) fn serialize_for_sdk_signing(
    action: &Action,
    nonce: u64,
    account: &Pubkey,
    out: &mut Vec<u8>,
) -> Result<()> {
    let tx_actions = action_to_tx_actions(action)?;
    if tx_actions.is_empty() {
        return Err(Error::EmptyOrders);
    }

    serialize_into_buffer(&tx_actions, out)?;
    out.extend_from_slice(&nonce.to_le_bytes());
    out.extend_from_slice(account.as_bytes());
    Ok(())
}

#[inline]
fn order_item_to_order_action(item: &OrderItem) -> Result<Option<TxAction>> {
    match item {
        OrderItem::Order(_) => order_item_to_tx_action(item).map(Some),
        _ => Ok(None),
    }
}

#[inline]
pub(crate) fn compute_order_item_id_with_seqno(
    item: &OrderItem,
    seqno: u32,
    nonce: u64,
    account: &Pubkey,
    scratch: &mut Vec<u8>,
) -> Option<Hash> {
    let action = order_item_to_order_action(item).ok()??;

    serialize_into_buffer(&action, scratch).ok()?;

    let mut hasher = Sha256::new();
    hasher.update(seqno.to_le_bytes());
    hasher.update(&*scratch);
    hasher.update(account.as_bytes());
    hasher.update(nonce.to_le_bytes());
    let hash: [u8; 32] = hasher.finalize().into();
    Some(Hash::from_bytes(hash))
}
