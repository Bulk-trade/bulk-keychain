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

    /// Compute SHA256 hash from arbitrary bytes
    ///
    /// **Note**: For computing order IDs, use `compute_order_id()` or
    /// `compute_limit_order_id()` / `compute_market_order_id()` instead.
    /// Those functions use the correct fixed-point serialization format
    /// that matches BULK's server-side order ID generation.
    ///
    /// This method is primarily used internally for legacy compatibility.
    #[inline]
    pub fn from_wincode_bytes(wincode_bytes: &[u8]) -> Self {
        use sha2::{Sha256, Digest};
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
            order_type: OrderType::market(),
            client_id: None,
        }
    }

    /// Set reduce-only flag
    pub fn reduce_only(mut self) -> Self {
        self.reduce_only = true;
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
// Order Item (union type)
// ============================================================================

/// An item in the orders array (can be an order, cancel, or cancel all)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderItem {
    /// Place a new order
    Order(Order),
    /// Cancel a specific order
    Cancel(Cancel),
    /// Cancel all orders
    CancelAll(CancelAll),
}

impl OrderItem {
    /// Get the discriminant for wincode serialization
    pub const fn discriminant(&self) -> u32 {
        match self {
            Self::Order(_) => 0,
            Self::Cancel(_) => 1,
            Self::CancelAll(_) => 2,
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

impl From<CancelAll> for OrderItem {
    fn from(cancel_all: CancelAll) -> Self {
        Self::CancelAll(cancel_all)
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
    /// Timestamp in milliseconds
    pub timestamp: u64,
    /// Asset symbol (e.g., "BTC")
    pub asset: String,
    /// Price
    pub price: f64,
}

// ============================================================================
// Action (main enum)
// ============================================================================

/// Transaction action type
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Order operations (place, cancel, cancel all)
    Order { orders: Vec<OrderItem> },
    /// Oracle price updates
    Oracle { oracles: Vec<OraclePrice> },
    /// Request testnet funds
    Faucet(Faucet),
    /// Update user settings
    UpdateUserSettings(UserSettings),
    /// Agent wallet management
    AgentWalletCreation(AgentWallet),
}

impl Action {
    /// Get the discriminant for wincode serialization
    pub const fn discriminant(&self) -> u32 {
        match self {
            Self::Order { .. } => 0,
            Self::Oracle { .. } => 1,
            Self::Faucet(_) => 2,
            Self::UpdateUserSettings(_) => 3,
            Self::AgentWalletCreation(_) => 4,
        }
    }

    /// Get the action type string for JSON
    pub const fn type_str(&self) -> &'static str {
        match self {
            Self::Order { .. } => "order",
            Self::Oracle { .. } => "oracle",
            Self::Faucet(_) => "faucet",
            Self::UpdateUserSettings(_) => "updateUserSettings",
            Self::AgentWalletCreation(_) => "agentWalletCreation",
        }
    }
}

// ============================================================================
// Signed Transaction
// ============================================================================

/// A signed transaction ready to submit to the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// The action being performed
    pub action: serde_json::Value,
    /// Account public key (base58)
    pub account: String,
    /// Signer public key (base58)
    pub signer: String,
    /// Signature (base58)
    pub signature: String,
    /// Pre-computed order/transaction ID (base58)
    /// This matches BULK's server-side ID generation: SHA256(wincode_bytes)
    #[serde(rename = "orderId", skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
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
