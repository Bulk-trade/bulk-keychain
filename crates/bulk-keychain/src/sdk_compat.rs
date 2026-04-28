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
            val.as_bytes().serialize(serializer)
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
            val.as_bytes().serialize(serializer)
        }
    }
}

mod serde_pubkey_vec {
    use super::*;

    pub fn serialize<S: Serializer>(
        vals: &[Pubkey],
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            vals.iter()
                .map(Pubkey::to_base58)
                .collect::<Vec<_>>()
                .serialize(serializer)
        } else {
            vals.iter()
                .map(Pubkey::as_bytes)
                .collect::<Vec<_>>()
                .serialize(serializer)
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

mod serde_opt_f64 {
    use super::*;

    pub fn serialize<S: Serializer>(
        val: &Option<f64>,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match val {
            None => serializer.serialize_none(),
            Some(v) => {
                if serializer.is_human_readable() {
                    serializer.serialize_str(&v.to_string())
                } else {
                    let fixed = (v * SCALE).round() as u64;
                    serializer.serialize_some(&fixed)
                }
            }
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
    #[serde(rename = "i", default)]
    iso: bool,
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
    #[serde(rename = "i", default)]
    iso: bool,
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
    #[serde(rename = "lim", with = "serde_opt_f64")]
    limit_price: Option<f64>,
    #[serde(rename = "i", default)]
    iso: bool,
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
    #[serde(rename = "lim", with = "serde_opt_f64")]
    limit_price: Option<f64>,
    #[serde(rename = "i", default)]
    iso: bool,
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
    #[serde(rename = "lmin", with = "serde_opt_f64")]
    limit_min: Option<f64>,
    #[serde(rename = "lmax", with = "serde_opt_f64")]
    limit_max: Option<f64>,
    #[serde(rename = "i", default)]
    iso: bool,
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
    #[serde(rename = "i", default)]
    iso: bool,
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
    #[serde(rename = "lim", with = "serde_opt_f64")]
    limit_price: Option<f64>,
    #[serde(rename = "i", default)]
    iso: bool,
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
#[serde(rename_all = "camelCase")]
struct TxCreateSubAccount {
    name: String,
    #[serde(default)]
    margin_symbol: Option<String>,
    #[serde(default)]
    margin_amount: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TxRemoveSubAccount {
    #[serde(with = "serde_pubkey")]
    to_remove: Pubkey,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TxRenameSubAccount {
    #[serde(with = "serde_pubkey", rename = "a")]
    account: Pubkey,
    #[serde(rename = "n")]
    name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum TxTransferKind {
    #[serde(rename = "internal")]
    Internal,
    #[serde(rename = "external")]
    External,
}

impl From<TransferKind> for TxTransferKind {
    #[inline]
    fn from(value: TransferKind) -> Self {
        match value {
            TransferKind::Internal => Self::Internal,
            TransferKind::External => Self::External,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TxTransfer {
    #[serde(rename = "k")]
    kind: TxTransferKind,
    #[serde(with = "serde_pubkey")]
    from: Pubkey,
    #[serde(with = "serde_pubkey")]
    to: Pubkey,
    margin_symbol: String,
    margin_amount: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TxCreateMultisig {
    #[serde(with = "serde_pubkey_vec")]
    signers: Vec<Pubkey>,
    threshold: u32,
    time_lock_secs: u32,
    proposal_lifetime_secs: u32,
}

#[derive(Clone, Debug, Serialize)]
struct TxMultisigPropose {
    #[serde(with = "serde_pubkey", rename = "m")]
    multisig: Pubkey,
    #[serde(rename = "a")]
    actions: Vec<TxAction>,
}

#[derive(Clone, Debug, Serialize)]
struct TxMultisigProposalRef {
    #[serde(with = "serde_pubkey", rename = "m")]
    multisig: Pubkey,
    #[serde(rename = "p")]
    proposal_id: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TxUpdateMultisigPolicy {
    #[serde(with = "serde_pubkey", rename = "m")]
    multisig: Pubkey,
    #[serde(with = "serde_pubkey_vec")]
    signers: Vec<Pubkey>,
    threshold: u32,
    time_lock_secs: u32,
    proposal_lifetime_secs: u32,
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
    #[serde(rename = "trl")]
    TrailingStop(TxTrailingStop),
    #[serde(rename = "of")]
    OnFill(TxOnFill),
    #[serde(rename = "px")]
    Price(TxPrice),
    #[allow(dead_code)]
    ReservedCorrs,
    #[serde(rename = "o")]
    PythOracle(TxPythOracle),
    #[allow(dead_code)]
    ReservedBeacon,
    #[allow(dead_code)]
    ReservedJoin,
    Faucet(TxFaucet),
    AgentWalletCreation(TxAgentWalletCreation),
    UpdateUserSettings(TxUpdateUserSettings),
    WhitelistFaucet(TxWhitelistFaucet),
    #[allow(dead_code)]
    Reserved20,
    #[allow(dead_code)]
    Reserved21,
    #[allow(dead_code)]
    Reserved22,
    #[allow(dead_code)]
    Reserved23,
    #[allow(dead_code)]
    Reserved24,
    #[allow(dead_code)]
    Reserved25,
    #[allow(dead_code)]
    Reserved26,
    #[serde(rename = "createSubAccount")]
    CreateSubAccount(TxCreateSubAccount),
    #[serde(rename = "removeSubAccount")]
    RemoveSubAccount(TxRemoveSubAccount),
    #[serde(rename = "transfer")]
    Transfer(TxTransfer),
    #[serde(rename = "createMultisig")]
    CreateMultisig(TxCreateMultisig),
    #[serde(rename = "msp")]
    MultisigPropose(TxMultisigPropose),
    #[serde(rename = "msa")]
    MultisigApprove(TxMultisigProposalRef),
    #[serde(rename = "msr")]
    MultisigReject(TxMultisigProposalRef),
    #[serde(rename = "msc")]
    MultisigCancel(TxMultisigProposalRef),
    #[serde(rename = "mse")]
    MultisigExecute(TxMultisigProposalRef),
    #[serde(rename = "msu")]
    UpdateMultisigPolicy(TxUpdateMultisigPolicy),
    #[serde(rename = "renameSubAccount")]
    RenameSubAccount(TxRenameSubAccount),
}

#[inline]
fn nan_to_none(v: f64) -> Option<f64> {
    if v.is_nan() {
        None
    } else {
        Some(v)
    }
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
                iso: order.iso,
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
                    iso: order.iso,
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
            limit_price: nan_to_none(stop.limit_price),
            iso: stop.iso,
        })),
        OrderItem::TakeProfit(tp) => Ok(TxAction::TakeProfit(TxTakeProfit {
            symbol: tp.symbol.clone(),
            is_buy: tp.is_buy,
            size: tp.size,
            trigger_price: tp.trigger_price,
            limit_price: nan_to_none(tp.limit_price),
            iso: tp.iso,
        })),
        OrderItem::RangeOco(rng) => Ok(TxAction::RangeOco(TxRangeOco {
            symbol: rng.symbol.clone(),
            is_buy: rng.is_buy,
            size: rng.size,
            collar_min: rng.collar_min,
            collar_max: rng.collar_max,
            limit_min: nan_to_none(rng.limit_min),
            limit_max: nan_to_none(rng.limit_max),
            iso: rng.iso,
        })),
        OrderItem::TriggerBasket(trig) => {
            let actions: Result<Vec<TxAction>> =
                trig.actions.iter().map(order_item_to_tx_action).collect();
            Ok(TxAction::TriggerBasket(TxTriggerBasket {
                symbol: trig.symbol.clone(),
                is_buy: trig.is_buy,
                trigger_price: trig.trigger_price,
                actions: actions?,
                iso: trig.iso,
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
            iso: trl.iso,
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
        Action::CreateSubAccount(action) => {
            Ok(vec![TxAction::CreateSubAccount(TxCreateSubAccount {
                name: action.name.clone(),
                margin_symbol: action.margin_symbol.clone(),
                margin_amount: action.margin_amount,
            })])
        }
        Action::RemoveSubAccount(action) => {
            Ok(vec![TxAction::RemoveSubAccount(TxRemoveSubAccount {
                to_remove: action.to_remove,
            })])
        }
        Action::RenameSubAccount(action) => {
            Ok(vec![TxAction::RenameSubAccount(TxRenameSubAccount {
                account: action.account,
                name: action.name.clone(),
            })])
        }
        Action::Transfer(transfer) => Ok(vec![TxAction::Transfer(TxTransfer {
            kind: TxTransferKind::from(transfer.kind),
            from: transfer.from,
            to: transfer.to,
            margin_symbol: transfer.margin_symbol.clone(),
            margin_amount: transfer.margin_amount,
        })]),
        Action::CreateMultisig(action) => Ok(vec![TxAction::CreateMultisig(TxCreateMultisig {
            signers: action.signers.clone(),
            threshold: action.threshold,
            time_lock_secs: action.time_lock_secs,
            proposal_lifetime_secs: action.proposal_lifetime_secs,
        })]),
        Action::MultisigPropose(action) => {
            let mut actions = Vec::new();
            for inner in &action.actions {
                actions.extend(action_to_tx_actions(inner)?);
            }
            Ok(vec![TxAction::MultisigPropose(TxMultisigPropose {
                multisig: action.multisig,
                actions,
            })])
        }
        Action::MultisigApprove(action) => {
            Ok(vec![TxAction::MultisigApprove(TxMultisigProposalRef {
                multisig: action.multisig,
                proposal_id: action.proposal_id,
            })])
        }
        Action::MultisigReject(action) => {
            Ok(vec![TxAction::MultisigReject(TxMultisigProposalRef {
                multisig: action.multisig,
                proposal_id: action.proposal_id,
            })])
        }
        Action::MultisigCancel(action) => {
            Ok(vec![TxAction::MultisigCancel(TxMultisigProposalRef {
                multisig: action.multisig,
                proposal_id: action.proposal_id,
            })])
        }
        Action::MultisigExecute(action) => {
            Ok(vec![TxAction::MultisigExecute(TxMultisigProposalRef {
                multisig: action.multisig,
                proposal_id: action.proposal_id,
            })])
        }
        Action::UpdateMultisigPolicy(action) => Ok(vec![TxAction::UpdateMultisigPolicy(
            TxUpdateMultisigPolicy {
                multisig: action.multisig,
                signers: action.signers.clone(),
                threshold: action.threshold,
                time_lock_secs: action.time_lock_secs,
                proposal_lifetime_secs: action.proposal_lifetime_secs,
            },
        )]),
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
