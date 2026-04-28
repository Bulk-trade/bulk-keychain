//! Type definitions for BULK transactions
//!
//! These types match the BULK exchange API specification exactly.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 32-byte public key (Ed25519)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Decode from base58 string
    pub fn from_base58(s: &str) -> crate::Result<Self> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| crate::Error::InvalidBase58(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(crate::Error::InvalidKeyLength {
                expected: 32,
                got: bytes.len(),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Encode to base58 string
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.0).into_string()
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}

impl Serialize for Pubkey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base58())
    }
}

impl<'de> Deserialize<'de> for Pubkey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Pubkey::from_base58(&s).map_err(serde::de::Error::custom)
    }
}

/// 32-byte hash (used for order IDs, client IDs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Decode from base58 string
    pub fn from_base58(s: &str) -> crate::Result<Self> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| crate::Error::InvalidBase58(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(crate::Error::InvalidHashLength(bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Encode to base58 string
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.0).into_string()
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Generate a random hash (useful for client order IDs)
    pub fn random() -> Self {
        use rand::Rng;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill(&mut bytes);
        Self(bytes)
    }

    /// Compute SHA256 hash from raw bytes.
    #[inline]
    pub fn from_wincode_bytes(wincode_bytes: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let hash: [u8; 32] = Sha256::digest(wincode_bytes).into();
        Self(hash)
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base58())
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Hash::from_base58(&s).map_err(serde::de::Error::custom)
    }
}

// ============================================================================
// Time In Force
// ============================================================================

/// Order time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeInForce {
    /// Good Till Cancel - rests on book until filled or cancelled
    Gtc,
    /// Immediate or Cancel - fill immediately or cancel
    Ioc,
    /// Add Liquidity Only - post-only, maker order
    Alo,
}

impl TimeInForce {
    /// Get the discriminant for wincode serialization
    pub const fn discriminant(&self) -> u32 {
        match self {
            Self::Gtc => 0,
            Self::Ioc => 1,
            Self::Alo => 2,
        }
    }
}

// ============================================================================
// Order Types
// ============================================================================

/// Order type (limit or trigger/market)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderType {
    /// Limit order with time-in-force
    Limit { tif: TimeInForce },
    /// Trigger/Market order
    Trigger {
        #[serde(rename = "isMarket")]
        is_market: bool,
        #[serde(rename = "triggerPx")]
        trigger_px: f64,
    },
}

impl OrderType {
    /// Create a limit order type
    pub const fn limit(tif: TimeInForce) -> Self {
        Self::Limit { tif }
    }

    /// Create a market order type (executes immediately at best price)
    pub const fn market() -> Self {
        Self::Trigger {
            is_market: true,
            trigger_px: 0.0,
        }
    }

    /// Get the discriminant for wincode serialization
    pub const fn discriminant(&self) -> u32 {
        match self {
            Self::Limit { .. } => 0,
            Self::Trigger { .. } => 1,
        }
    }
}

// ============================================================================
// Order
// ============================================================================

/// A trading order
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    /// Market symbol (e.g., "BTC-USD")
    #[serde(rename = "c")]
    pub symbol: String,
    /// Buy (true) or Sell (false)
    #[serde(rename = "b")]
    pub is_buy: bool,
    /// Price (0.0 for market orders)
    #[serde(rename = "px")]
    pub price: f64,
    /// Size/Quantity
    #[serde(rename = "sz")]
    pub size: f64,
    /// Reduce-only flag
    #[serde(rename = "r")]
    pub reduce_only: bool,
    /// Isolated-margin flag
    #[serde(rename = "i", default)]
    pub iso: bool,
    /// Order type
    #[serde(rename = "t")]
    pub order_type: OrderType,
    /// Client order ID (optional)
    #[serde(rename = "cloid", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<Hash>,
}

impl Order {
    /// Create a new limit order
    pub fn limit(
        symbol: impl Into<String>,
        is_buy: bool,
        price: f64,
        size: f64,
        tif: TimeInForce,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            is_buy,
            price,
            size,
            reduce_only: false,
            iso: false,
            order_type: OrderType::limit(tif),
            client_id: None,
        }
    }

    /// Create a market order
    pub fn market(symbol: impl Into<String>, is_buy: bool, size: f64) -> Self {
        Self {
            symbol: symbol.into(),
            is_buy,
            price: 0.0,
            size,
            reduce_only: false,
            iso: false,
            order_type: OrderType::market(),
            client_id: None,
        }
    }

    /// Set reduce-only flag
    pub fn reduce_only(mut self) -> Self {
        self.reduce_only = true;
        self
    }

    /// Set isolated-margin flag
    pub fn isolated(mut self) -> Self {
        self.iso = true;
        self
    }

    /// Set client order ID
    pub fn with_client_id(mut self, client_id: Hash) -> Self {
        self.client_id = Some(client_id);
        self
    }

    /// Generate and set a random client order ID
    pub fn with_random_client_id(mut self) -> Self {
        self.client_id = Some(Hash::random());
        self
    }
}

// ============================================================================
// Cancel
// ============================================================================

/// Cancel a specific order by ID
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cancel {
    /// Market symbol
    #[serde(rename = "c")]
    pub symbol: String,
    /// Order ID to cancel
    #[serde(rename = "oid")]
    pub order_id: Hash,
}

impl Cancel {
    /// Create a new cancel request
    pub fn new(symbol: impl Into<String>, order_id: Hash) -> Self {
        Self {
            symbol: symbol.into(),
            order_id,
        }
    }
}

/// Modify an existing order
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modify {
    /// Order ID to modify
    #[serde(rename = "oid")]
    pub order_id: Hash,
    /// Market symbol
    pub symbol: String,
    /// New amount/size
    pub amount: f64,
}

impl Modify {
    /// Create a new modify request
    pub fn new(order_id: Hash, symbol: impl Into<String>, amount: f64) -> Self {
        Self {
            order_id,
            symbol: symbol.into(),
            amount,
        }
    }
}

// ============================================================================
// Cancel All
// ============================================================================

/// Cancel all orders (optionally filtered by symbols)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelAll {
    /// Symbols to cancel orders for (empty = all symbols)
    #[serde(rename = "c")]
    pub symbols: Vec<String>,
}

impl CancelAll {
    /// Cancel all orders across all symbols
    pub fn all() -> Self {
        Self { symbols: vec![] }
    }

    /// Cancel all orders for specific symbols
    pub fn for_symbols(symbols: Vec<String>) -> Self {
        Self { symbols }
    }
}

// ============================================================================
// Conditional order types
// ============================================================================

/// Stop-loss order: triggers when price crosses threshold
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    pub symbol: String,
    /// true = buy/long side, false = sell/short side
    pub is_buy: bool,
    pub size: f64,
    pub trigger_price: f64,
    /// Limit price; NaN means market-style fill
    pub limit_price: f64,
    /// Isolated-margin flag
    #[serde(default)]
    pub iso: bool,
}

/// Take-profit order: triggers when price crosses threshold
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TakeProfit {
    pub symbol: String,
    /// true = buy/long side, false = sell/short side
    pub is_buy: bool,
    pub size: f64,
    pub trigger_price: f64,
    /// Limit price; NaN means market-style fill
    pub limit_price: f64,
    /// Isolated-margin flag
    #[serde(default)]
    pub iso: bool,
}

/// Range / OCO order: collar around a position
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeOco {
    pub symbol: String,
    /// true = buy/long collar, false = sell/short collar
    pub is_buy: bool,
    pub size: f64,
    pub collar_min: f64,
    pub collar_max: f64,
    /// Limit price for min side; NaN means market-style fill
    pub limit_min: f64,
    /// Limit price for max side; NaN means market-style fill
    pub limit_max: f64,
    /// Isolated-margin flag
    #[serde(default)]
    pub iso: bool,
}

/// Trigger basket: fires a set of actions when price crosses threshold.
/// Nested actions may be: m, l, mod, cx, cxa, st, tp, rng.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerBasket {
    pub symbol: String,
    /// true = buy/long side, false = sell/short side
    pub is_buy: bool,
    pub trigger_price: f64,
    pub actions: Vec<OrderItem>,
    /// Isolated-margin flag
    #[serde(default)]
    pub iso: bool,
}

/// Trailing stop: protective stop that follows price by a fixed distance in bps,
/// resetting forward on favorable moves in increments of `step_bps`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrailingStop {
    pub symbol: String,
    /// Protected position direction (true = long, false = short)
    pub is_buy: bool,
    pub size: f64,
    /// Trailing distance in basis points
    pub trail_bps: u32,
    /// Favorable reset step in basis points
    pub step_bps: u32,
    /// Optional triggered limit price; None means market-style trigger
    pub limit_price: Option<f64>,
    /// Isolated-margin flag
    #[serde(default)]
    pub iso: bool,
}

/// On-fill consequent: one-shot follow-up actions executed on first fill of a parent action.
/// `p` is the parent action's seqno (index) in the same transaction.
/// Allowed consequent types: m, l, mod, cx, cxa, st, tp, rng, trig, trl.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnFill {
    /// Parent action seqno (0-based index in the transaction).
    pub p: u32,
    /// One-shot consequent actions executed on first fill of the parent.
    pub actions: Vec<OrderItem>,
}

// ============================================================================
// Order Item (union type)
// ============================================================================

/// An item in the orders array
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderItem {
    /// Place a new order
    Order(Order),
    /// Modify an existing order size
    Modify(Modify),
    /// Cancel a specific order
    Cancel(Cancel),
    /// Cancel all orders
    CancelAll(CancelAll),
    /// Stop-loss conditional order
    Stop(Stop),
    /// Take-profit conditional order
    TakeProfit(TakeProfit),
    /// Range / OCO collar order
    RangeOco(RangeOco),
    /// Trigger basket: fires nested actions when price crosses threshold
    TriggerBasket(TriggerBasket),
    /// On-fill consequent: one-shot follow-up actions on first fill of a parent action
    OnFill(OnFill),
    /// Trailing stop: protective stop that follows price by a fixed bps distance
    TrailingStop(TrailingStop),
}

impl OrderItem {
    /// Get the discriminant for wincode serialization
    pub const fn discriminant(&self) -> u32 {
        match self {
            Self::Order(order) => match order.order_type {
                OrderType::Limit { .. } => 1,   // l
                OrderType::Trigger { .. } => 0, // m
            },
            Self::Modify(_) => 2,        // mod
            Self::Cancel(_) => 3,        // cx
            Self::CancelAll(_) => 4,     // cxa
            Self::Stop(_) => 5,          // st
            Self::TakeProfit(_) => 6,    // tp
            Self::RangeOco(_) => 7,      // rng
            Self::TriggerBasket(_) => 8, // trig
            Self::TrailingStop(_) => 9,  // trl
            Self::OnFill(_) => 10,       // of
        }
    }
}

impl From<Order> for OrderItem {
    fn from(order: Order) -> Self {
        Self::Order(order)
    }
}

impl From<Cancel> for OrderItem {
    fn from(cancel: Cancel) -> Self {
        Self::Cancel(cancel)
    }
}

impl From<Modify> for OrderItem {
    fn from(modify: Modify) -> Self {
        Self::Modify(modify)
    }
}

impl From<CancelAll> for OrderItem {
    fn from(cancel_all: CancelAll) -> Self {
        Self::CancelAll(cancel_all)
    }
}

impl From<Stop> for OrderItem {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

impl From<TakeProfit> for OrderItem {
    fn from(tp: TakeProfit) -> Self {
        Self::TakeProfit(tp)
    }
}

impl From<RangeOco> for OrderItem {
    fn from(rng: RangeOco) -> Self {
        Self::RangeOco(rng)
    }
}

impl From<TriggerBasket> for OrderItem {
    fn from(trig: TriggerBasket) -> Self {
        Self::TriggerBasket(trig)
    }
}

impl From<OnFill> for OrderItem {
    fn from(of: OnFill) -> Self {
        Self::OnFill(of)
    }
}

impl From<TrailingStop> for OrderItem {
    fn from(trl: TrailingStop) -> Self {
        Self::TrailingStop(trl)
    }
}

// ============================================================================
// Faucet
// ============================================================================

/// Request testnet funds
#[derive(Debug, Clone, PartialEq)]
pub struct Faucet {
    /// User to receive funds
    pub user: Pubkey,
    /// Amount (optional, defaults to 10,000)
    pub amount: Option<f64>,
}

impl Faucet {
    /// Create a new faucet request
    pub fn new(user: Pubkey) -> Self {
        Self { user, amount: None }
    }

    /// Create a faucet request with specific amount
    pub fn with_amount(user: Pubkey, amount: f64) -> Self {
        Self {
            user,
            amount: Some(amount),
        }
    }
}

// ============================================================================
// Agent Wallet
// ============================================================================

/// Register or remove an agent wallet
#[derive(Debug, Clone, PartialEq)]
pub struct AgentWallet {
    /// Agent public key
    pub agent: Pubkey,
    /// Delete flag (true = remove, false = add)
    pub delete: bool,
}

impl AgentWallet {
    /// Add an agent wallet
    pub fn add(agent: Pubkey) -> Self {
        Self {
            agent,
            delete: false,
        }
    }

    /// Remove an agent wallet
    pub fn remove(agent: Pubkey) -> Self {
        Self {
            agent,
            delete: true,
        }
    }
}

// ============================================================================
// User Settings
// ============================================================================

/// Update user settings (leverage)
#[derive(Debug, Clone, PartialEq)]
pub struct UserSettings {
    /// Max leverage per symbol: [(symbol, leverage), ...]
    pub max_leverage: Vec<(String, f64)>,
}

impl UserSettings {
    /// Create new user settings
    pub fn new(max_leverage: Vec<(String, f64)>) -> Self {
        Self { max_leverage }
    }

    /// Set leverage for a single symbol
    pub fn set_leverage(symbol: impl Into<String>, leverage: f64) -> Self {
        Self {
            max_leverage: vec![(symbol.into(), leverage)],
        }
    }
}

// ============================================================================
// Oracle
// ============================================================================

/// Oracle price update (permissioned)
#[derive(Debug, Clone, PartialEq)]
pub struct OraclePrice {
    /// Timestamp
    pub timestamp: u64,
    /// Asset symbol (e.g., "BTC")
    pub asset: String,
    /// Price
    pub price: f64,
}

/// Pyth oracle price entry (admin `o` action)
#[derive(Debug, Clone, PartialEq)]
pub struct PythOraclePrice {
    /// Timestamp
    pub timestamp: u64,
    /// Feed index
    pub feed_index: u64,
    /// Raw price integer
    pub price: u64,
    /// Decimal exponent
    pub exponent: i16,
}

/// Whitelist/un-whitelist an account for faucet access (admin)
#[derive(Debug, Clone, PartialEq)]
pub struct WhitelistFaucet {
    /// Target account pubkey
    pub target: Pubkey,
    /// true = whitelist, false = un-whitelist
    pub whitelist: bool,
}

// ============================================================================
// Create Sub Account
// ============================================================================

/// Create a named sub-account under the signing master account, with an
/// optional initial margin transfer.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateSubAccount {
    /// Sub-account display name
    pub name: String,
    /// Optional margin asset symbol. Must be present when `margin_amount` is non-zero.
    pub margin_symbol: Option<String>,
    /// Optional initial margin amount. Default 0.0
    pub margin_amount: Option<f64>,
}

impl CreateSubAccount {
    /// Create a sub-account with no initial margin transfer.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            margin_symbol: None,
            margin_amount: None,
        }
    }

    /// Create a sub-account with an initial margin transfer.
    pub fn with_margin(
        name: impl Into<String>,
        margin_symbol: impl Into<String>,
        margin_amount: f64,
    ) -> Self {
        Self {
            name: name.into(),
            margin_symbol: Some(margin_symbol.into()),
            margin_amount: Some(margin_amount),
        }
    }
}

// ============================================================================
// Remove Sub Account
// ============================================================================

/// Remove a sub-account belonging to the signing master account.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveSubAccount {
    pub to_remove: Pubkey,
}

impl RemoveSubAccount {
    pub fn new(to_remove: Pubkey) -> Self {
        Self { to_remove }
    }
}

// ============================================================================
// Rename Sub Account
// ============================================================================

/// Rename a sub-account belonging to the signing master account.
#[derive(Debug, Clone, PartialEq)]
pub struct RenameSubAccount {
    pub account: Pubkey,
    pub name: String,
}

impl RenameSubAccount {
    pub fn new(account: Pubkey, name: impl Into<String>) -> Self {
        Self {
            account,
            name: name.into(),
        }
    }
}

// ============================================================================
// Transfer
// ============================================================================

/// Direction of a margin transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferKind {
    /// Between two accounts inside BULK.
    #[default]
    Internal,
    /// To/from an external destination.
    External,
}

/// Transfer margin between accounts.
#[derive(Debug, Clone, PartialEq)]
pub struct Transfer {
    pub kind: TransferKind,
    pub from: Pubkey,
    pub to: Pubkey,
    pub margin_symbol: String,
    pub margin_amount: f64,
}

impl Transfer {
    /// Internal transfer between two BULK accounts.
    pub fn internal(
        from: Pubkey,
        to: Pubkey,
        margin_symbol: impl Into<String>,
        margin_amount: f64,
    ) -> Self {
        Self {
            kind: TransferKind::Internal,
            from,
            to,
            margin_symbol: margin_symbol.into(),
            margin_amount,
        }
    }

    /// External transfer in/out of BULK.
    pub fn external(
        from: Pubkey,
        to: Pubkey,
        margin_symbol: impl Into<String>,
        margin_amount: f64,
    ) -> Self {
        Self {
            kind: TransferKind::External,
            from,
            to,
            margin_symbol: margin_symbol.into(),
            margin_amount,
        }
    }
}

// ============================================================================
// Multisig
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct CreateMultisig {
    pub signers: Vec<Pubkey>,
    pub threshold: u32,
    pub time_lock_secs: u32,
    pub proposal_lifetime_secs: u32,
}

impl CreateMultisig {
    pub fn new(signers: Vec<Pubkey>, threshold: u32) -> Self {
        Self {
            signers,
            threshold,
            time_lock_secs: 0,
            proposal_lifetime_secs: 7 * 24 * 3600,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultisigPropose {
    pub multisig: Pubkey,
    pub actions: Vec<Action>,
}

impl MultisigPropose {
    pub fn new(multisig: Pubkey, actions: Vec<Action>) -> Self {
        Self { multisig, actions }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultisigApprove {
    pub multisig: Pubkey,
    pub proposal_id: u64,
}

impl MultisigApprove {
    pub fn new(multisig: Pubkey, proposal_id: u64) -> Self {
        Self {
            multisig,
            proposal_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultisigReject {
    pub multisig: Pubkey,
    pub proposal_id: u64,
}

impl MultisigReject {
    pub fn new(multisig: Pubkey, proposal_id: u64) -> Self {
        Self {
            multisig,
            proposal_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultisigCancel {
    pub multisig: Pubkey,
    pub proposal_id: u64,
}

impl MultisigCancel {
    pub fn new(multisig: Pubkey, proposal_id: u64) -> Self {
        Self {
            multisig,
            proposal_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultisigExecute {
    pub multisig: Pubkey,
    pub proposal_id: u64,
}

impl MultisigExecute {
    pub fn new(multisig: Pubkey, proposal_id: u64) -> Self {
        Self {
            multisig,
            proposal_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateMultisigPolicy {
    pub multisig: Pubkey,
    pub signers: Vec<Pubkey>,
    pub threshold: u32,
    pub time_lock_secs: u32,
    pub proposal_lifetime_secs: u32,
}

impl UpdateMultisigPolicy {
    pub fn new(multisig: Pubkey, signers: Vec<Pubkey>, threshold: u32) -> Self {
        Self {
            multisig,
            signers,
            threshold,
            time_lock_secs: 0,
            proposal_lifetime_secs: 7 * 24 * 3600,
        }
    }
}

// ============================================================================
// Action (main enum)
// ============================================================================

/// Transaction action type
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Order operations (place, cancel, cancel all)
    Order { orders: Vec<OrderItem> },
    /// Oracle price updates (`px`)
    Oracle { oracles: Vec<OraclePrice> },
    /// Batch Pyth oracle updates (`o`)
    PythOracle { oracles: Vec<PythOraclePrice> },
    /// Request testnet funds
    Faucet(Faucet),
    /// Update user settings
    UpdateUserSettings(UserSettings),
    /// Agent wallet management
    AgentWalletCreation(AgentWallet),
    /// Whitelist faucet access for an account (admin)
    WhitelistFaucet(WhitelistFaucet),
    /// Create a named sub-account (optional initial margin transfer)
    CreateSubAccount(CreateSubAccount),
    /// Remove a sub-account
    RemoveSubAccount(RemoveSubAccount),
    /// Rename a sub-account
    RenameSubAccount(RenameSubAccount),
    /// Margin transfer between accounts
    Transfer(Transfer),
    /// Create a multisig account
    CreateMultisig(CreateMultisig),
    /// Propose one or more actions for a multisig account
    MultisigPropose(MultisigPropose),
    /// Approve a multisig proposal
    MultisigApprove(MultisigApprove),
    /// Reject a multisig proposal
    MultisigReject(MultisigReject),
    /// Cancel a multisig proposal
    MultisigCancel(MultisigCancel),
    /// Execute a multisig proposal
    MultisigExecute(MultisigExecute),
    /// Update a multisig policy
    UpdateMultisigPolicy(UpdateMultisigPolicy),
}

impl Action {
    /// Get the discriminant for wincode serialization
    pub const fn discriminant(&self) -> u32 {
        match self {
            Self::Order { .. } => 0,  // container variant, not a wire discriminant
            Self::Oracle { .. } => 5, // px
            Self::PythOracle { .. } => 6,
            Self::Faucet(_) => 7,
            Self::UpdateUserSettings(_) => 9,
            Self::AgentWalletCreation(_) => 8,
            Self::WhitelistFaucet(_) => 10,
            Self::CreateSubAccount(_) => 27,
            Self::RemoveSubAccount(_) => 28,
            Self::Transfer(_) => 29,
            Self::CreateMultisig(_) => 30,
            Self::MultisigPropose(_) => 31,
            Self::MultisigApprove(_) => 32,
            Self::MultisigReject(_) => 33,
            Self::MultisigCancel(_) => 34,
            Self::MultisigExecute(_) => 35,
            Self::UpdateMultisigPolicy(_) => 36,
            Self::RenameSubAccount(_) => 37,
        }
    }

    /// Get the action type string for JSON
    pub const fn type_str(&self) -> &'static str {
        match self {
            Self::Order { .. } => "order",
            Self::Oracle { .. } => "px",
            Self::PythOracle { .. } => "o",
            Self::Faucet(_) => "faucet",
            Self::UpdateUserSettings(_) => "updateUserSettings",
            Self::AgentWalletCreation(_) => "agentWalletCreation",
            Self::WhitelistFaucet(_) => "whitelistFaucet",
            Self::CreateSubAccount(_) => "createSubAccount",
            Self::RemoveSubAccount(_) => "removeSubAccount",
            Self::Transfer(_) => "transfer",
            Self::CreateMultisig(_) => "createMultisig",
            Self::MultisigPropose(_) => "msp",
            Self::MultisigApprove(_) => "msa",
            Self::MultisigReject(_) => "msr",
            Self::MultisigCancel(_) => "msc",
            Self::MultisigExecute(_) => "mse",
            Self::UpdateMultisigPolicy(_) => "msu",
            Self::RenameSubAccount(_) => "renameSubAccount",
        }
    }
}

impl From<RenameSubAccount> for Action {
    fn from(action: RenameSubAccount) -> Self {
        Self::RenameSubAccount(action)
    }
}

impl From<CreateMultisig> for Action {
    fn from(action: CreateMultisig) -> Self {
        Self::CreateMultisig(action)
    }
}

impl From<MultisigPropose> for Action {
    fn from(action: MultisigPropose) -> Self {
        Self::MultisigPropose(action)
    }
}

impl From<MultisigApprove> for Action {
    fn from(action: MultisigApprove) -> Self {
        Self::MultisigApprove(action)
    }
}

impl From<MultisigReject> for Action {
    fn from(action: MultisigReject) -> Self {
        Self::MultisigReject(action)
    }
}

impl From<MultisigCancel> for Action {
    fn from(action: MultisigCancel) -> Self {
        Self::MultisigCancel(action)
    }
}

impl From<MultisigExecute> for Action {
    fn from(action: MultisigExecute) -> Self {
        Self::MultisigExecute(action)
    }
}

impl From<UpdateMultisigPolicy> for Action {
    fn from(action: UpdateMultisigPolicy) -> Self {
        Self::UpdateMultisigPolicy(action)
    }
}

// ============================================================================
// Signed Transaction
// ============================================================================

/// A signed transaction ready to submit to the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// Actions to execute atomically (compact tagged format)
    pub actions: Vec<serde_json::Value>,
    /// Transaction nonce
    pub nonce: u64,
    /// Account public key (base58)
    pub account: String,
    /// Signer public key (base58)
    pub signer: String,
    /// Signature (base58)
    pub signature: String,
    /// Optional pre-computed order ID for client-side optimistic tracking.
    /// This is not part of the API request payload.
    #[serde(skip_serializing, skip_deserializing, default)]
    pub order_id: Option<String>,
    /// Optional pre-computed order IDs for multi-order transactions.
    /// This is not part of the API request payload.
    #[serde(skip_serializing, skip_deserializing, default)]
    pub order_ids: Option<Vec<String>>,
}

impl SignedTransaction {
    /// Serialize to JSON string
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string(self).map_err(crate::Error::from)
    }

    /// Serialize to JSON bytes
    pub fn to_json_bytes(&self) -> crate::Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(crate::Error::from)
    }
}
