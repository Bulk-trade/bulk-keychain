//! Canonical BULK-SDK-compatible serialization.

use crate::types::*;
use crate::{Error, Result};
use serde::ser::{SerializeStruct, SerializeTuple};
use serde::Serialize;
use serde::Serializer;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const SCALE: f64 = 1e8;
const SIGNABLE_ACTIONS_V2_PREFIX: &[u8; 21] = b"\xff\xff\xff\xff\xff\xff\xff\xffbulk-actions\x02";
const SIGNABLE_ACTIONS_V3_PREFIX: &[u8; 21] = b"\xff\xff\xff\xff\xff\xff\xff\xffbulk-actions\x03";
const SIGNABLE_ACTIONS_V4_PREFIX: &[u8; 21] = b"\xff\xff\xff\xff\xff\xff\xff\xffbulk-actions\x04";

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

struct SafeF64(f64);

impl Serialize for SafeF64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serde_safe_f64::serialize(&self.0, serializer)
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
struct TxCommission {
    #[serde(with = "serde_pubkey")]
    to: Pubkey,
    fee: u8,
}

impl TryFrom<Commission> for TxCommission {
    type Error = Error;

    fn try_from(value: Commission) -> Result<Self> {
        if value.fee == 0 || value.fee > MAX_COMMISSION_FEE_BPS {
            return Err(Error::InvalidOrder(
                "builder-code fee must be 1..=15 bps".to_string(),
            ));
        }
        Ok(Self {
            to: value.to,
            fee: value.fee,
        })
    }
}

// The SDK preserves a distinct pre-slippage persisted layout inside legacy multisigs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SigningLayout {
    Legacy,
    LegacyEmbedded,
    V2,
    V3,
    V4,
}

#[derive(Clone, Debug)]
struct TxMarketOrder {
    symbol: String,
    is_buy: bool,
    size: f64,
    reduce_only: bool,
    iso: bool,
    commission: Option<TxCommission>,
    slippage: Option<f64>,
    layout: SigningLayout,
}

// V2 applies to the complete action array, including orders without explicit slippage.
impl Serialize for TxMarketOrder {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let mut state = serializer.serialize_struct(
                "MarketOrder",
                5 + usize::from(self.commission.is_some()) + usize::from(self.slippage.is_some()),
            )?;
            state.serialize_field("c", &self.symbol)?;
            state.serialize_field("b", &self.is_buy)?;
            state.serialize_field("sz", &SafeF64(self.size))?;
            state.serialize_field("r", &self.reduce_only)?;
            state.serialize_field("i", &self.iso)?;
            if let Some(commission) = &self.commission {
                state.serialize_field("builderCode", commission)?;
            }
            if let Some(slippage) = self.slippage {
                state.serialize_field("slippage", &SafeF64(slippage))?;
            }
            state.end()
        } else if matches!(self.layout, SigningLayout::V2 | SigningLayout::V3) {
            let mut tuple = serializer.serialize_tuple(7)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&self.reduce_only)?;
            tuple.serialize_element(&self.iso)?;
            tuple.serialize_element(&self.commission)?;
            tuple.serialize_element(&self.slippage.map(SafeF64))?;
            tuple.end()
        } else {
            let mut tuple = serializer.serialize_tuple(
                5 + usize::from(
                    self.layout == SigningLayout::LegacyEmbedded || self.commission.is_some(),
                ),
            )?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&self.reduce_only)?;
            tuple.serialize_element(&self.iso)?;
            if self.layout == SigningLayout::LegacyEmbedded || self.commission.is_some() {
                tuple.serialize_element(&self.commission)?;
            }
            tuple.end()
        }
    }
}

#[derive(Clone, Debug)]
struct TxLimitOrder {
    symbol: String,
    is_buy: bool,
    price: f64,
    size: f64,
    tif: TxTimeInForce,
    reduce_only: bool,
    iso: bool,
    commission: Option<TxCommission>,
    layout: SigningLayout,
}

impl Serialize for TxLimitOrder {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(
            "TxLimitOrder",
            7 + usize::from(self.layout != SigningLayout::Legacy || self.commission.is_some()),
        )?;
        state.serialize_field("c", &self.symbol)?;
        state.serialize_field("b", &self.is_buy)?;
        state.serialize_field("px", &SafeF64(self.price))?;
        state.serialize_field("sz", &SafeF64(self.size))?;
        state.serialize_field("tif", &self.tif)?;
        state.serialize_field("r", &self.reduce_only)?;
        state.serialize_field("i", &self.iso)?;
        if self.layout != SigningLayout::Legacy || self.commission.is_some() {
            state.serialize_field("commission", &self.commission)?;
        }
        state.end()
    }
}

#[derive(Clone, Debug, Serialize)]
struct TxOrderHashMarketOrder {
    #[serde(rename = "c")]
    symbol: String,
    #[serde(rename = "b")]
    is_buy: bool,
    #[serde(rename = "sz", with = "serde_safe_f64")]
    size: f64,
    #[serde(rename = "r")]
    reduce_only: bool,
    #[serde(rename = "i")]
    iso: bool,
    #[serde(
        rename = "slippage",
        with = "serde_opt_f64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    slippage: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct TxOrderHashLimitOrder {
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
    #[serde(rename = "i")]
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

#[derive(Clone, Debug)]
struct TxStop {
    symbol: String,
    is_buy: bool,
    size: f64,
    trigger_price: f64,
    limit_price: Option<f64>,
    iso: bool,
    commission: Option<TxCommission>,
    slippage: Option<f64>,
    layout: SigningLayout,
}

impl Serialize for TxStop {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let mut state =
                serializer.serialize_struct("Stop", 6 + usize::from(self.commission.is_some()))?;
            state.serialize_field("c", &self.symbol)?;
            state.serialize_field("d", &self.is_buy)?;
            state.serialize_field("sz", &SafeF64(self.size))?;
            state.serialize_field("tr", &SafeF64(self.trigger_price))?;
            state.serialize_field("lim", &self.limit_price.map(SafeF64))?;
            state.serialize_field("i", &self.iso)?;
            if let Some(commission) = &self.commission {
                state.serialize_field("builderCode", commission)?;
            }
            state.end()
        } else if matches!(self.layout, SigningLayout::V4) {
            let mut tuple = serializer.serialize_tuple(8)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&SafeF64(self.trigger_price))?;
            tuple.serialize_element(&self.limit_price.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.serialize_element(&self.commission)?;
            tuple.serialize_element(&self.slippage.map(SafeF64))?;
            tuple.end()
        } else if matches!(self.layout, SigningLayout::V2 | SigningLayout::V3) {
            let mut tuple = serializer.serialize_tuple(7)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&SafeF64(self.trigger_price))?;
            tuple.serialize_element(&self.limit_price.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.serialize_element(&self.commission)?;
            tuple.end()
        } else {
            let mut tuple = serializer.serialize_tuple(6)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&SafeF64(self.trigger_price))?;
            tuple.serialize_element(&self.limit_price.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.end()
        }
    }
}

#[derive(Clone, Debug)]
struct TxTakeProfit {
    symbol: String,
    is_buy: bool,
    size: f64,
    trigger_price: f64,
    limit_price: Option<f64>,
    iso: bool,
    commission: Option<TxCommission>,
    slippage: Option<f64>,
    layout: SigningLayout,
}

impl Serialize for TxTakeProfit {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        TxStop {
            symbol: self.symbol.clone(),
            is_buy: self.is_buy,
            size: self.size,
            trigger_price: self.trigger_price,
            limit_price: self.limit_price,
            iso: self.iso,
            commission: self.commission.clone(),
            slippage: self.slippage,
            layout: self.layout,
        }
        .serialize(serializer)
    }
}

#[derive(Clone, Debug)]
struct TxRangeOco {
    symbol: String,
    is_buy: bool,
    size: f64,
    collar_min: f64,
    collar_max: f64,
    limit_min: Option<f64>,
    limit_max: Option<f64>,
    iso: bool,
    commission: Option<TxCommission>,
    sl_slippage: Option<f64>,
    tp_slippage: Option<f64>,
    layout: SigningLayout,
}

impl Serialize for TxRangeOco {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let mut state =
                serializer.serialize_struct("Range", 8 + usize::from(self.commission.is_some()))?;
            state.serialize_field("c", &self.symbol)?;
            state.serialize_field("d", &self.is_buy)?;
            state.serialize_field("sz", &SafeF64(self.size))?;
            state.serialize_field("pmin", &SafeF64(self.collar_min))?;
            state.serialize_field("pmax", &SafeF64(self.collar_max))?;
            state.serialize_field("lmin", &self.limit_min.map(SafeF64))?;
            state.serialize_field("lmax", &self.limit_max.map(SafeF64))?;
            state.serialize_field("i", &self.iso)?;
            if let Some(commission) = &self.commission {
                state.serialize_field("builderCode", commission)?;
            }
            state.end()
        } else if matches!(self.layout, SigningLayout::V4) {
            let mut tuple = serializer.serialize_tuple(11)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&SafeF64(self.collar_min))?;
            tuple.serialize_element(&SafeF64(self.collar_max))?;
            tuple.serialize_element(&self.limit_min.map(SafeF64))?;
            tuple.serialize_element(&self.limit_max.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.serialize_element(&self.commission)?;
            tuple.serialize_element(&self.sl_slippage.map(SafeF64))?;
            tuple.serialize_element(&self.tp_slippage.map(SafeF64))?;
            tuple.end()
        } else if matches!(self.layout, SigningLayout::V2 | SigningLayout::V3) {
            let mut tuple = serializer.serialize_tuple(9)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&SafeF64(self.collar_min))?;
            tuple.serialize_element(&SafeF64(self.collar_max))?;
            tuple.serialize_element(&self.limit_min.map(SafeF64))?;
            tuple.serialize_element(&self.limit_max.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.serialize_element(&self.commission)?;
            tuple.end()
        } else {
            let mut tuple = serializer.serialize_tuple(8)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&SafeF64(self.collar_min))?;
            tuple.serialize_element(&SafeF64(self.collar_max))?;
            tuple.serialize_element(&self.limit_min.map(SafeF64))?;
            tuple.serialize_element(&self.limit_max.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.end()
        }
    }
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
    trigger: Box<TxAction>,
    #[serde(rename = "actions")]
    actions: Vec<TxAction>,
}

#[derive(Clone, Debug)]
struct TxTrailingStop {
    symbol: String,
    is_buy: bool,
    size: f64,
    trail_bps: u32,
    step_bps: u32,
    limit_price: Option<f64>,
    iso: bool,
    commission: Option<TxCommission>,
    slippage: Option<f64>,
    layout: SigningLayout,
}

impl Serialize for TxTrailingStop {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let mut state = serializer
                .serialize_struct("Trailing", 7 + usize::from(self.commission.is_some()))?;
            state.serialize_field("c", &self.symbol)?;
            state.serialize_field("b", &self.is_buy)?;
            state.serialize_field("sz", &SafeF64(self.size))?;
            state.serialize_field("trb", &self.trail_bps)?;
            state.serialize_field("stb", &self.step_bps)?;
            state.serialize_field("lim", &self.limit_price.map(SafeF64))?;
            state.serialize_field("i", &self.iso)?;
            if let Some(commission) = &self.commission {
                state.serialize_field("builderCode", commission)?;
            }
            state.end()
        } else if matches!(self.layout, SigningLayout::V4) {
            let mut tuple = serializer.serialize_tuple(9)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&self.trail_bps)?;
            tuple.serialize_element(&self.step_bps)?;
            tuple.serialize_element(&self.limit_price.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.serialize_element(&self.commission)?;
            tuple.serialize_element(&self.slippage.map(SafeF64))?;
            tuple.end()
        } else if matches!(self.layout, SigningLayout::V2 | SigningLayout::V3) {
            let mut tuple = serializer.serialize_tuple(8)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&self.trail_bps)?;
            tuple.serialize_element(&self.step_bps)?;
            tuple.serialize_element(&self.limit_price.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.serialize_element(&self.commission)?;
            tuple.end()
        } else {
            let mut tuple = serializer.serialize_tuple(7)?;
            tuple.serialize_element(&self.symbol)?;
            tuple.serialize_element(&self.is_buy)?;
            tuple.serialize_element(&SafeF64(self.size))?;
            tuple.serialize_element(&self.trail_bps)?;
            tuple.serialize_element(&self.step_bps)?;
            tuple.serialize_element(&self.limit_price.map(SafeF64))?;
            tuple.serialize_element(&self.iso)?;
            tuple.end()
        }
    }
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
    max_leverage: BTreeMap<String, f64>,
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
    #[serde(rename = "l")]
    proposal_lifetime_secs: Option<u32>,
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
struct TxApproveCommissionFee {
    #[serde(with = "serde_pubkey", rename = "to")]
    to: Pubkey,
    #[serde(rename = "fee")]
    max_fee: u8,
}

#[derive(Clone, Debug, Serialize)]
struct TxRevokeCommissionFee {
    #[serde(with = "serde_pubkey", rename = "to")]
    to: Pubkey,
}

#[derive(Clone, Debug, Serialize)]
struct TxLiquidatorInstrumentConfig {
    symbol: String,
    max_exposure: f64,
    reserve: f64,
    rfactor: f64,
    volume_percent: f64,
    volume_min: f64,
    volume_rampup: u64,
    max_sweep_bps: f64,
    max_adl_notional: f64,
    max_adl_percent: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TxLiquidatorConfig {
    cross_exposure: f64,
    scoring_skew: f64,
    toxicity: f64,
    urgency_size_fraction: f64,
    sweep_sds: f64,
    instruments: Vec<TxLiquidatorInstrumentConfig>,
}

#[derive(Clone, Debug, Serialize)]
enum TxAction {
    MarketOrder(TxMarketOrder),
    LimitOrder(TxLimitOrder),
    ModifyOrder(TxModifyOrder),
    Cancel(TxCancelOrder),
    CancelAll(TxCancelAll),
    Stop(TxStop),
    TakeProfit(TxTakeProfit),
    RangeOco(TxRangeOco),
    TriggerBasket(TxTriggerBasket),
    TrailingStop(TxTrailingStop),
    OnFill(TxOnFill),
    Price(TxPrice),
    #[allow(dead_code)]
    ReservedCorrs,
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
    #[allow(dead_code)]
    Reserved38,
    #[allow(dead_code)]
    Reserved39,
    ApproveCommissionFee(TxApproveCommissionFee),
    RevokeCommissionFee(TxRevokeCommissionFee),
    #[allow(dead_code)]
    Reserved42,
    #[serde(rename = "liq")]
    UpdateLiquidatorConfig(TxLiquidatorConfig),
    #[allow(dead_code)]
    Reserved44,
    #[allow(dead_code)]
    Reserved45,
    #[allow(dead_code)]
    Reserved46,
    #[allow(dead_code)]
    Reserved47,
    #[allow(dead_code)]
    Reserved48,
    #[allow(dead_code)]
    Reserved49,
    #[allow(dead_code)]
    Reserved50,
    #[allow(dead_code)]
    Reserved51,
    #[allow(dead_code)]
    Reserved52,
    #[allow(dead_code)]
    Reserved53,
    #[allow(dead_code)]
    Reserved54,
}

#[derive(Clone, Debug, Serialize)]
enum TxOrderHashAction {
    #[serde(rename = "m")]
    MarketOrder(TxOrderHashMarketOrder),
    #[serde(rename = "l")]
    LimitOrder(TxOrderHashLimitOrder),
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
fn order_item_to_tx_action(item: &OrderItem, layout: SigningLayout) -> Result<TxAction> {
    match item {
        OrderItem::Order(order) => match order.order_type {
            OrderType::Limit { .. } if order.slippage.is_some() => Err(Error::InvalidOrder(
                "slippage is only supported on market orders".to_string(),
            )),
            OrderType::Limit { tif } => Ok(TxAction::LimitOrder(TxLimitOrder {
                symbol: order.symbol.clone(),
                is_buy: order.is_buy,
                price: order.price,
                size: order.size,
                tif: TxTimeInForce::from(tif),
                reduce_only: order.reduce_only,
                iso: order.iso,
                commission: order.commission.map(TxCommission::try_from).transpose()?,
                layout,
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
                    commission: order.commission.map(TxCommission::try_from).transpose()?,
                    layout,
                    slippage: order.slippage,
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
            commission: stop.commission.map(TxCommission::try_from).transpose()?,
            slippage: stop.slippage,
            layout,
        })),
        OrderItem::TakeProfit(tp) => Ok(TxAction::TakeProfit(TxTakeProfit {
            symbol: tp.symbol.clone(),
            is_buy: tp.is_buy,
            size: tp.size,
            trigger_price: tp.trigger_price,
            limit_price: nan_to_none(tp.limit_price),
            iso: tp.iso,
            commission: tp.commission.map(TxCommission::try_from).transpose()?,
            slippage: tp.slippage,
            layout,
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
            commission: rng.commission.map(TxCommission::try_from).transpose()?,
            sl_slippage: rng.sl_slippage,
            tp_slippage: rng.tp_slippage,
            layout,
        })),
        OrderItem::TriggerBasket(trig) => {
            let actions: Result<Vec<TxAction>> = trig
                .actions
                .iter()
                .map(|item| order_item_to_tx_action(item, layout))
                .collect();
            Ok(TxAction::TriggerBasket(TxTriggerBasket {
                symbol: trig.symbol.clone(),
                is_buy: trig.is_buy,
                trigger_price: trig.trigger_price,
                actions: actions?,
            }))
        }
        OrderItem::OnFill(of) => {
            if !matches!(of.trigger.as_ref(), OrderItem::Order(_)) {
                return Err(Error::InvalidOrder(
                    "on-fill trigger must be a market or limit order".to_string(),
                ));
            }
            let trigger = Box::new(order_item_to_tx_action(&of.trigger, layout)?);
            let actions: Result<Vec<TxAction>> = of
                .actions
                .iter()
                .map(|item| order_item_to_tx_action(item, layout))
                .collect();
            Ok(TxAction::OnFill(TxOnFill {
                trigger,
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
            commission: trl.commission.map(TxCommission::try_from).transpose()?,
            slippage: trl.slippage,
            layout,
        })),
    }
}

#[inline]
fn action_to_tx_actions(action: &Action, layout: SigningLayout) -> Result<Vec<TxAction>> {
    match action {
        Action::Order { orders } => orders
            .iter()
            .map(|item| order_item_to_tx_action(item, layout))
            .collect(),
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
            let mut max_leverage = BTreeMap::new();
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
            margin_amount: transfer.margin_amount,
        })]),
        Action::Withdraw(_) | Action::WithdrawLockRecover(_) => Err(Error::LegacyWithdrawal),
        Action::CreateMultisig(action) => Ok(vec![TxAction::CreateMultisig(TxCreateMultisig {
            signers: action.signers.clone(),
            threshold: action.threshold,
            time_lock_secs: action.time_lock_secs,
            proposal_lifetime_secs: action.proposal_lifetime_secs,
        })]),
        Action::MultisigPropose(action) => {
            let mut actions = Vec::new();
            for inner in &action.actions {
                actions.extend(action_to_tx_actions(
                    inner,
                    if matches!(layout, SigningLayout::V2 | SigningLayout::V3) {
                        layout
                    } else {
                        SigningLayout::LegacyEmbedded
                    },
                )?);
            }
            Ok(vec![TxAction::MultisigPropose(TxMultisigPropose {
                multisig: action.multisig,
                actions,
                proposal_lifetime_secs: action.proposal_lifetime_secs,
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
        Action::ApproveCommissionFee(action) => {
            if action.max_fee == 0 || action.max_fee > MAX_COMMISSION_FEE_BPS {
                return Err(Error::InvalidOrder(
                    "builder-code fee must be 1..=15 bps".to_string(),
                ));
            }
            Ok(vec![TxAction::ApproveCommissionFee(
                TxApproveCommissionFee {
                    to: action.to,
                    max_fee: action.max_fee,
                },
            )])
        }
        Action::RevokeCommissionFee(action) => {
            Ok(vec![TxAction::RevokeCommissionFee(TxRevokeCommissionFee {
                to: action.to,
            })])
        }
        Action::UpdateLiquidatorConfig(config) => {
            Ok(vec![TxAction::UpdateLiquidatorConfig(TxLiquidatorConfig {
                cross_exposure: config.cross_exposure,
                scoring_skew: config.scoring_skew,
                toxicity: config.toxicity,
                urgency_size_fraction: config.urgency_size_fraction,
                sweep_sds: config.sweep_sds,
                instruments: config
                    .sorted_instruments()
                    .into_iter()
                    .map(|i| TxLiquidatorInstrumentConfig {
                        symbol: i.symbol.clone(),
                        max_exposure: i.max_exposure,
                        reserve: i.reserve,
                        rfactor: i.rfactor,
                        volume_percent: i.volume_percent,
                        volume_min: i.volume_min,
                        volume_rampup: i.volume_rampup,
                        max_sweep_bps: i.max_sweep_bps,
                        max_adl_notional: i.max_adl_notional,
                        max_adl_percent: i.max_adl_percent,
                    })
                    .collect(),
            })])
        }
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
    signature_domain: SignatureDomain,
    nonce: u64,
    account: &Pubkey,
    out: &mut Vec<u8>,
) -> Result<()> {
    let layout = if action_contains_leaf_conditional(action) {
        SigningLayout::V4
    } else if action_has_builder_code(action) {
        SigningLayout::V3
    } else if action_has_explicit_slippage(action) {
        SigningLayout::V2
    } else {
        SigningLayout::Legacy
    };
    let tx_actions = action_to_tx_actions(action, layout)?;
    if tx_actions.is_empty() {
        return Err(Error::EmptyOrders);
    }

    out.clear();
    match layout {
        SigningLayout::V2 => out.extend_from_slice(SIGNABLE_ACTIONS_V2_PREFIX),
        SigningLayout::V3 => out.extend_from_slice(SIGNABLE_ACTIONS_V3_PREFIX),
        SigningLayout::V4 => out.extend_from_slice(SIGNABLE_ACTIONS_V4_PREFIX),
        SigningLayout::Legacy | SigningLayout::LegacyEmbedded => {}
    }
    bincode::serialize_into(&mut *out, &tx_actions)
        .map_err(|e| Error::SerializationError(e.to_string()))?;
    out.extend_from_slice(&nonce.to_le_bytes());
    out.extend_from_slice(account.as_bytes());
    out.push(signature_domain as u8);
    Ok(())
}

fn action_contains_leaf_conditional(action: &Action) -> bool {
    match action {
        Action::Order { orders } => orders.iter().any(order_item_contains_leaf_conditional),
        Action::MultisigPropose(proposal) => proposal
            .actions
            .iter()
            .any(action_contains_leaf_conditional),
        _ => false,
    }
}

fn order_item_contains_leaf_conditional(item: &OrderItem) -> bool {
    match item {
        OrderItem::Stop(_)
        | OrderItem::TakeProfit(_)
        | OrderItem::RangeOco(_)
        | OrderItem::TrailingStop(_) => true,
        OrderItem::TriggerBasket(trigger) => trigger
            .actions
            .iter()
            .any(order_item_contains_leaf_conditional),
        OrderItem::OnFill(on_fill) => {
            order_item_contains_leaf_conditional(&on_fill.trigger)
                || on_fill
                    .actions
                    .iter()
                    .any(order_item_contains_leaf_conditional)
        }
        _ => false,
    }
}

fn action_has_explicit_slippage(action: &Action) -> bool {
    match action {
        Action::Order { orders } => orders.iter().any(order_item_has_explicit_slippage),
        Action::MultisigPropose(proposal) => {
            proposal.actions.iter().any(action_has_explicit_slippage)
        }
        _ => false,
    }
}

fn action_has_builder_code(action: &Action) -> bool {
    match action {
        Action::Order { orders } => orders.iter().any(order_item_has_builder_code),
        Action::MultisigPropose(proposal) => proposal.actions.iter().any(action_has_builder_code),
        _ => false,
    }
}

fn order_item_has_builder_code(item: &OrderItem) -> bool {
    match item {
        OrderItem::Order(order) => order.commission.is_some(),
        OrderItem::Stop(order) => order.commission.is_some(),
        OrderItem::TakeProfit(order) => order.commission.is_some(),
        OrderItem::RangeOco(order) => order.commission.is_some(),
        OrderItem::TrailingStop(order) => order.commission.is_some(),
        OrderItem::TriggerBasket(trigger) => {
            trigger.actions.iter().any(order_item_has_builder_code)
        }
        OrderItem::OnFill(on_fill) => {
            order_item_has_builder_code(&on_fill.trigger)
                || on_fill.actions.iter().any(order_item_has_builder_code)
        }
        _ => false,
    }
}

fn order_item_has_explicit_slippage(item: &OrderItem) -> bool {
    match item {
        OrderItem::Order(order) => order.slippage.is_some(),
        OrderItem::TriggerBasket(trigger) => {
            trigger.actions.iter().any(order_item_has_explicit_slippage)
        }
        OrderItem::OnFill(on_fill) => {
            order_item_has_explicit_slippage(&on_fill.trigger)
                || on_fill.actions.iter().any(order_item_has_explicit_slippage)
        }
        _ => false,
    }
}

#[inline]
fn order_item_to_order_hash_action(item: &OrderItem) -> Result<Option<TxOrderHashAction>> {
    match item {
        OrderItem::Order(order) => match order.order_type {
            OrderType::Limit { tif } => {
                Ok(Some(TxOrderHashAction::LimitOrder(TxOrderHashLimitOrder {
                    symbol: order.symbol.clone(),
                    is_buy: order.is_buy,
                    price: order.price,
                    size: order.size,
                    tif: TxTimeInForce::from(tif),
                    reduce_only: order.reduce_only,
                    iso: order.iso,
                })))
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
                Ok(Some(TxOrderHashAction::MarketOrder(
                    TxOrderHashMarketOrder {
                        symbol: order.symbol.clone(),
                        is_buy: order.is_buy,
                        size: order.size,
                        reduce_only: order.reduce_only,
                        iso: order.iso,
                        slippage: order.slippage,
                    },
                )))
            }
        },
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
    let action = order_item_to_order_hash_action(item).ok()??;

    serialize_into_buffer(&action, scratch).ok()?;

    let mut hasher = Sha256::new();
    hasher.update(seqno.to_le_bytes());
    hasher.update(&*scratch);
    hasher.update(account.as_bytes());
    hasher.update(nonce.to_le_bytes());
    let hash: [u8; 32] = hasher.finalize().into();
    Some(Hash::from_bytes(hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_signable_bytes_end_with_the_selected_network_domain() {
        let account = Pubkey::from_bytes([0xa5; 32]);
        let action = Action::Faucet(Faucet::new(account));
        let mut canonical_without_domain =
            bincode::serialize(&action_to_tx_actions(&action, SigningLayout::Legacy).unwrap())
                .unwrap();
        canonical_without_domain.extend_from_slice(&7u64.to_le_bytes());
        canonical_without_domain.extend_from_slice(account.as_bytes());
        let mut actual = Vec::with_capacity(128);

        for (domain, byte) in [
            (SignatureDomain::Mainnet, 1),
            (SignatureDomain::Testnet, 2),
            (SignatureDomain::Devnet, 3),
        ] {
            serialize_for_sdk_signing(&action, domain, 7, &account, &mut actual).unwrap();
            assert_eq!(
                &actual[..canonical_without_domain.len()],
                canonical_without_domain
            );
            assert_eq!(actual.len(), canonical_without_domain.len() + 1);
            assert_eq!(actual.last(), Some(&byte));
        }
    }

    #[test]
    fn market_order_slippage_is_optional_and_changes_signed_bytes() {
        let account = Pubkey::from_bytes([0x5a; 32]);
        let nonce = 42;
        let without_slippage = Action::Order {
            orders: vec![Order::market("BTC-USD", true, 0.25).into()],
        };
        let with_slippage = Action::Order {
            orders: vec![Order::market("BTC-USD", true, 0.25)
                .with_slippage(25.5)
                .into()],
        };

        let without_json = serde_json::to_string(
            &action_to_tx_actions(&without_slippage, SigningLayout::Legacy).unwrap(),
        )
        .unwrap();
        let with_json = serde_json::to_string(
            &action_to_tx_actions(&with_slippage, SigningLayout::V2).unwrap(),
        )
        .unwrap();
        assert!(!without_json.contains("slippage"));
        assert!(with_json.contains("\"slippage\":\"25.5\""));

        let mut without_bytes = Vec::new();
        let mut with_bytes = Vec::new();
        serialize_for_sdk_signing(
            &without_slippage,
            SignatureDomain::Devnet,
            nonce,
            &account,
            &mut without_bytes,
        )
        .unwrap();
        serialize_for_sdk_signing(
            &with_slippage,
            SignatureDomain::Devnet,
            nonce,
            &account,
            &mut with_bytes,
        )
        .unwrap();
        assert_ne!(without_bytes, with_bytes);
        assert!(with_bytes.starts_with(SIGNABLE_ACTIONS_V2_PREFIX));
        assert_eq!(
            with_bytes.len(),
            without_bytes.len()
                + SIGNABLE_ACTIONS_V2_PREFIX.len()
                + 2 * std::mem::size_of::<u8>()
                + std::mem::size_of::<u64>()
        );
    }

    fn first_action_discriminant(action: &Action) -> u32 {
        let mut out = Vec::with_capacity(128);
        serialize_for_sdk_signing(
            action,
            SignatureDomain::Devnet,
            7,
            &Pubkey::from_bytes([1u8; 32]),
            &mut out,
        )
        .unwrap();
        u32::from_le_bytes(out[8..12].try_into().unwrap())
    }

    fn make_liq_config() -> LiquidatorConfig {
        let inst = |symbol: &str, max_exposure: f64| LiquidatorInstrumentConfig {
            symbol: symbol.to_string(),
            max_exposure,
            reserve: 50.0,
            rfactor: 0.25,
            volume_percent: 5.0,
            volume_min: 1.0,
            volume_rampup: 0,
            max_sweep_bps: 100.0,
            max_adl_notional: 0.0,
            max_adl_percent: 0.0,
        };
        LiquidatorConfig::new(15e6, 0.5, 0.0, 0.25, 2.0)
            .with_instrument(inst("BTC-USD", 10e6))
            .with_instrument(inst("ETH-USD", 5e6))
    }

    #[test]
    fn on_fill_limit_trigger_with_nested_market_order_golden_vector() {
        let trigger: OrderItem =
            Order::limit("BTC-USD", true, 100_000.0, 0.1, TimeInForce::Gtc).into();
        let consequent: OrderItem = Order::market("ETH-USD", false, 1.25).into();
        let action = Action::Order {
            orders: vec![OrderItem::OnFill(OnFill {
                trigger: Box::new(trigger.clone()),
                actions: vec![consequent.clone()],
            })],
        };
        let account = Pubkey::from_bytes([7u8; 32]);
        let nonce = 1_234_567_890;
        let mut actual = Vec::new();
        serialize_for_sdk_signing(
            &action,
            SignatureDomain::Devnet,
            nonce,
            &account,
            &mut actual,
        )
        .unwrap();

        let expected = hex::decode(
            "01000000000000000a0000000100000007000000000000004254432d5553440100a0724e18090000809698000000000000000000000001000000000000000000000007000000000000004554482d5553440040597307000000000000d202964900000000070707070707070707070707070707070707070707070707070707070707070703",
        )
        .unwrap();

        assert_eq!(actual, expected);
        assert_eq!(u64::from_le_bytes(actual[0..8].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(actual[8..12].try_into().unwrap()), 10);
        assert_eq!(u32::from_le_bytes(actual[12..16].try_into().unwrap()), 1);
        assert_eq!(u64::from_le_bytes(actual[54..62].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(actual[62..66].try_into().unwrap()), 0);
        assert_eq!(
            u64::from_le_bytes(actual[92..100].try_into().unwrap()),
            1_234_567_890
        );
        assert_eq!(&actual[100..132], &[7u8; 32]);
        assert_eq!(actual[132], SignatureDomain::Devnet as u8);

        let mut prepared_trigger = Vec::new();
        let mut prepared_consequent = Vec::new();
        serialize_for_sdk_signing(
            &Action::Order {
                orders: vec![trigger],
            },
            SignatureDomain::Devnet,
            nonce,
            &account,
            &mut prepared_trigger,
        )
        .unwrap();
        serialize_for_sdk_signing(
            &Action::Order {
                orders: vec![consequent],
            },
            SignatureDomain::Devnet,
            nonce,
            &account,
            &mut prepared_consequent,
        )
        .unwrap();
        let trigger_action_end = prepared_trigger.len() - 41;
        let consequent_action_end = prepared_consequent.len() - 41;
        let mut bulkx_reference = Vec::new();
        bulkx_reference.extend_from_slice(&1u64.to_le_bytes());
        bulkx_reference.extend_from_slice(&10u32.to_le_bytes());
        bulkx_reference.extend_from_slice(&prepared_trigger[8..trigger_action_end]);
        bulkx_reference.extend_from_slice(&1u64.to_le_bytes());
        bulkx_reference.extend_from_slice(&prepared_consequent[8..consequent_action_end]);
        bulkx_reference.extend_from_slice(&prepared_trigger[trigger_action_end..]);

        assert_eq!(actual, bulkx_reference);
    }

    #[test]
    fn on_fill_rejects_non_order_trigger_before_signing() {
        let action = Action::Order {
            orders: vec![OrderItem::OnFill(OnFill {
                trigger: Box::new(OrderItem::Stop(Stop {
                    symbol: "BTC-USD".to_string(),
                    is_buy: false,
                    size: 0.1,
                    trigger_price: 90_000.0,
                    limit_price: f64::NAN,
                    iso: false,
                    commission: None,
                    slippage: None,
                })),
                actions: vec![Order::market("BTC-USD", false, 0.1).into()],
            })],
        };
        let mut actual = Vec::new();

        assert!(matches!(
            serialize_for_sdk_signing(
                &action,
                SignatureDomain::Devnet,
                1_234_567_890,
                &Pubkey::from_bytes([7u8; 32]),
                &mut actual,
            ),
            Err(Error::InvalidOrder(message))
                if message == "on-fill trigger must be a market or limit order"
        ));
    }

    #[test]
    fn liquidator_config_discriminant_and_layout_match_sdk() {
        let action = Action::UpdateLiquidatorConfig(make_liq_config());
        assert_eq!(first_action_discriminant(&action), 43);

        let account = Pubkey::from_base58("4zvwRjXUKGfvwnParsHAS3HuSVzV5cA4McphgmoCtajS").unwrap();
        let mut out = Vec::new();
        serialize_for_sdk_signing(&action, SignatureDomain::Devnet, 42, &account, &mut out)
            .unwrap();

        assert_eq!(out.len(), 275);

        let f64_at = |o: usize| f64::from_le_bytes(out[o..o + 8].try_into().unwrap());
        let u64_at = |o: usize| u64::from_le_bytes(out[o..o + 8].try_into().unwrap());

        assert_eq!(u64_at(0), 1);
        assert_eq!(f64_at(12), 15e6);
        assert_eq!(f64_at(20), 0.5);
        assert_eq!(f64_at(28), 0.0);
        assert_eq!(f64_at(36), 0.25);
        assert_eq!(f64_at(44), 2.0);
        assert_eq!(u64_at(52), 2);

        assert_eq!(u64_at(60), 7);
        assert_eq!(&out[68..75], b"BTC-USD");
        assert_eq!(f64_at(75), 10e6);
        assert_eq!(f64_at(83), 50.0);
        assert_eq!(f64_at(91), 0.25);
        assert_eq!(f64_at(99), 5.0);
        assert_eq!(f64_at(107), 1.0);
        assert_eq!(u64_at(115), 0);
        assert_eq!(f64_at(123), 100.0);
        assert_eq!(f64_at(131), 0.0);
        assert_eq!(f64_at(139), 0.0);

        assert_eq!(&out[155..162], b"ETH-USD");
        assert_eq!(f64_at(162), 5e6);

        assert_eq!(u64_at(234), 42);
        assert_eq!(&out[242..274], account.as_bytes());
        assert_eq!(out[274], SignatureDomain::Devnet as u8);
    }

    #[test]
    fn liquidator_config_signs_to_the_sdk_reference_signature() {
        let keypair = crate::Keypair::from_secret_key(&[0u8; 32]).unwrap();
        assert_eq!(
            keypair.pubkey().to_base58(),
            "4zvwRjXUKGfvwnParsHAS3HuSVzV5cA4McphgmoCtajS"
        );

        let mut signer = crate::Signer::new(keypair, SignatureDomain::Devnet);
        let signed = signer
            .sign_update_liquidator_config(make_liq_config(), Some(42))
            .unwrap();

        assert_eq!(
            signed.signature,
            "3kCYJmqzxfj4iotJ2kDMvkE5YY1EutzSoHdiLNpSGc89ryjaoRDKZXjHpWR59Eoxzk5vgR2ASSaE8QrtjH4FQ5PY"
        );
    }

    #[test]
    fn liquidator_instruments_serialize_in_sorted_symbol_order() {
        let account = Pubkey::from_bytes([9u8; 32]);
        let serialize = |config: LiquidatorConfig| {
            let mut out = Vec::new();
            let action = Action::UpdateLiquidatorConfig(config);
            serialize_for_sdk_signing(&action, SignatureDomain::Devnet, 42, &account, &mut out)
                .unwrap();
            out
        };

        let mut reversed = make_liq_config();
        reversed.instruments.reverse();
        assert_eq!(serialize(reversed), serialize(make_liq_config()));
    }

    #[test]
    fn commission_action_discriminants_match_sdk() {
        let recipient = Pubkey::from_bytes([2u8; 32]);

        assert_eq!(
            first_action_discriminant(&Action::ApproveCommissionFee(ApproveCommissionFee {
                to: recipient,
                max_fee: 5,
            })),
            40
        );
        assert_eq!(
            first_action_discriminant(&Action::RevokeCommissionFee(RevokeCommissionFee {
                to: recipient,
            })),
            41
        );
    }

    #[test]
    fn builder_code_uses_v3_signing_and_preserves_order_hash() {
        let account = Pubkey::from_bytes([3u8; 32]);
        let recipient = Pubkey::from_bytes([4u8; 32]);
        let plain = OrderItem::Order(Order::limit(
            "BTC-USD",
            true,
            100000.0,
            0.1,
            TimeInForce::Gtc,
        ));
        let commissioned = OrderItem::Order(
            Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc)
                .with_commission(recipient, 5)
                .unwrap(),
        );
        let mut plain_bytes = Vec::with_capacity(256);
        let mut commissioned_bytes = Vec::with_capacity(256);
        let mut scratch = Vec::with_capacity(96);

        serialize_for_sdk_signing(
            &Action::Order {
                orders: vec![plain.clone()],
            },
            SignatureDomain::Devnet,
            9,
            &account,
            &mut plain_bytes,
        )
        .unwrap();
        serialize_for_sdk_signing(
            &Action::Order {
                orders: vec![commissioned.clone()],
            },
            SignatureDomain::Devnet,
            9,
            &account,
            &mut commissioned_bytes,
        )
        .unwrap();

        assert_ne!(plain_bytes, commissioned_bytes);
        assert!(commissioned_bytes.starts_with(SIGNABLE_ACTIONS_V3_PREFIX));
        assert_eq!(
            compute_order_item_id_with_seqno(&plain, 0, 9, &account, &mut scratch),
            compute_order_item_id_with_seqno(&commissioned, 0, 9, &account, &mut scratch)
        );
    }

    #[test]
    fn conditional_builder_code_uses_v4_and_binds_the_conditional() {
        let account = Pubkey::from_bytes([3u8; 32]);
        let recipient = Pubkey::from_bytes([4u8; 32]);
        let plain = Stop {
            symbol: "BTC-USD".to_string(),
            is_buy: false,
            size: 0.1,
            trigger_price: 90_000.0,
            limit_price: f64::NAN,
            iso: false,
            commission: None,
            slippage: None,
        };
        let mut commissioned = plain.clone();
        commissioned.commission = Some(Commission::new(recipient, 5).unwrap());
        let action = |stop| Action::Order {
            orders: vec![OrderItem::Stop(stop)],
        };
        let mut plain_bytes = Vec::new();
        let mut builder_bytes = Vec::new();
        serialize_for_sdk_signing(
            &action(plain),
            SignatureDomain::Devnet,
            9,
            &account,
            &mut plain_bytes,
        )
        .unwrap();
        serialize_for_sdk_signing(
            &action(commissioned),
            SignatureDomain::Devnet,
            9,
            &account,
            &mut builder_bytes,
        )
        .unwrap();
        assert!(!plain_bytes.starts_with(SIGNABLE_ACTIONS_V3_PREFIX));
        assert!(builder_bytes.starts_with(SIGNABLE_ACTIONS_V4_PREFIX));
        assert_ne!(plain_bytes, builder_bytes);
    }
}
