//! Native Node.js bindings for BULK transaction signing
//!
//! This module provides high-performance native bindings using NAPI-RS.
//! It's significantly faster than pure JavaScript or WASM implementations.

use bulk_keychain::{
    Cancel, CancelAll, Hash, Keypair, NonceManager, NonceStrategy, Order, OrderItem,
    OrderType, PreparedMessage, Pubkey, Signer, TimeInForce, UserSettings,
    prepare_all, prepare_group, prepare_message, prepare_agent_wallet, prepare_faucet,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Deserialize;

// ============================================================================
// Keypair
// ============================================================================

/// Ed25519 keypair for signing transactions
#[napi]
pub struct NativeKeypair {
    inner: Keypair,
}

#[napi]
impl NativeKeypair {
    /// Generate a new random keypair
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Keypair::generate(),
        }
    }

    /// Create from base58-encoded secret key or full keypair
    #[napi(factory)]
    pub fn from_base58(s: String) -> Result<Self> {
        let inner = Keypair::from_base58(&s)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create from raw bytes (32-byte secret or 64-byte full keypair)
    #[napi(factory)]
    pub fn from_bytes(bytes: Buffer) -> Result<Self> {
        let inner = Keypair::from_bytes(&bytes)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Get the public key as base58 string
    #[napi(getter)]
    pub fn pubkey(&self) -> String {
        self.inner.pubkey().to_base58()
    }

    /// Get the full keypair as base58 (64 bytes)
    #[napi]
    pub fn to_base58(&self) -> String {
        self.inner.to_base58()
    }

    /// Get the full keypair as bytes (64 bytes)
    #[napi]
    pub fn to_bytes(&self) -> Buffer {
        Buffer::from(self.inner.to_bytes().to_vec())
    }

    /// Get the secret key as bytes (32 bytes)
    #[napi]
    pub fn secret_key(&self) -> Buffer {
        Buffer::from(self.inner.secret_key().to_vec())
    }

    /// Clone the keypair
    #[napi]
    pub fn clone_keypair(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for NativeKeypair {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Signer
// ============================================================================

/// High-performance transaction signer
#[napi]
pub struct NativeSigner {
    inner: Signer,
}

#[napi]
impl NativeSigner {
    /// Create a new signer from a keypair
    #[napi(constructor)]
    pub fn new(keypair: &NativeKeypair) -> Self {
        Self {
            inner: Signer::new(keypair.inner.clone()),
        }
    }

    /// Create a signer from base58-encoded secret key
    #[napi(factory)]
    pub fn from_base58(s: String) -> Result<Self> {
        let keypair = Keypair::from_base58(&s)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner: Signer::new(keypair),
        })
    }

    /// Create a signer with nonce management
    #[napi(factory)]
    pub fn with_nonce_manager(keypair: &NativeKeypair, strategy: String) -> Result<Self> {
        let nonce_strategy = match strategy.as_str() {
            "timestamp" => NonceStrategy::Timestamp,
            "counter" => NonceStrategy::Counter,
            "highFrequency" => NonceStrategy::TimestampWithCounter,
            _ => return Err(Error::from_reason(
                "Invalid nonce strategy. Use 'timestamp', 'counter', or 'highFrequency'",
            )),
        };
        let nonce_manager = NonceManager::new(nonce_strategy);
        Ok(Self {
            inner: Signer::with_nonce_manager(keypair.inner.clone(), nonce_manager),
        })
    }

    /// Get the signer's public key
    #[napi(getter)]
    pub fn pubkey(&self) -> String {
        self.inner.pubkey().to_base58()
    }

    // ========================================================================
    // Simplified API
    // ========================================================================

    /// Sign a single order/cancel/cancelAll
    ///
    /// Most common use case - returns a single signed transaction.
    ///
    /// @example
    /// ```typescript
    /// const signed = signer.sign({ type: 'order', symbol: 'BTC-USD', isBuy: true, price: 100000, size: 0.1 });
    /// ```
    #[napi]
    pub fn sign(
        &mut self,
        order: OrderInput,
        nonce: Option<f64>,
    ) -> Result<SignedTransactionOutput> {
        let order_item: OrderItem = order.try_into()?;
        let nonce_val = nonce.map(|n| n as u64);
        
        let signed = self
            .inner
            .sign(order_item, nonce_val)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(signed.into())
    }

    /// Sign multiple orders - each becomes its own transaction (parallel)
    ///
    /// each order gets independent confirmation/rejection.
    /// Automatically parallelizes when > 10 orders.
    ///
    /// @example
    /// ```typescript
    /// const orders = [order1, order2, order3];
    /// const signedTxs = signer.signAll(orders); // Returns SignedTransaction[]
    /// ```
    #[napi]
    pub fn sign_all(
        &self,
        orders: Vec<OrderInput>,
        base_nonce: Option<f64>,
    ) -> Result<Vec<SignedTransactionOutput>> {
        let order_items: Result<Vec<OrderItem>> = orders.into_iter().map(|o| o.try_into()).collect();
        let order_items = order_items?;

        let base = base_nonce.map(|n| n as u64);
        let signed = self
            .inner
            .sign_all(order_items, base)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(signed.into_iter().map(Into::into).collect())
    }

    /// Sign multiple orders atomically in ONE transaction
    ///
    /// Use for bracket orders (entry + stop loss + take profit) where
    /// all orders must succeed or fail together.
    ///
    /// @example
    /// ```typescript
    /// const bracket = [entryOrder, stopLoss, takeProfit];
    /// const signed = signer.signGroup(bracket); // Returns single SignedTransaction
    /// ```
    #[napi]
    pub fn sign_group(
        &mut self,
        orders: Vec<OrderInput>,
        nonce: Option<f64>,
    ) -> Result<SignedTransactionOutput> {
        let order_items: Result<Vec<OrderItem>> = orders.into_iter().map(|o| o.try_into()).collect();
        let order_items = order_items?;

        let nonce_val = nonce.map(|n| n as u64);
        let signed = self
            .inner
            .sign_group(order_items, nonce_val)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(signed.into())
    }

    // ========================================================================
    // Other signing methods
    // ========================================================================

    /// Sign a faucet request (testnet only)
    #[napi]
    pub fn sign_faucet(&mut self, nonce: Option<f64>) -> Result<SignedTransactionOutput> {
        let nonce_val = nonce.map(|n| n as u64);
        let signed = self
            .inner
            .sign_faucet(nonce_val)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(signed.into())
    }

    /// Sign agent wallet creation/deletion
    #[napi]
    pub fn sign_agent_wallet(
        &mut self,
        agent_pubkey: String,
        delete: bool,
        nonce: Option<f64>,
    ) -> Result<SignedTransactionOutput> {
        let agent = Pubkey::from_base58(&agent_pubkey)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_agent_wallet(agent, delete, nonce_val)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(signed.into())
    }

    /// Sign user settings update
    #[napi]
    pub fn sign_user_settings(
        &mut self,
        max_leverage: Vec<LeverageSetting>,
        nonce: Option<f64>,
    ) -> Result<SignedTransactionOutput> {
        let leverage_vec: Vec<(String, f64)> = max_leverage
            .into_iter()
            .map(|l| (l.symbol, l.leverage))
            .collect();
        let user_settings = UserSettings::new(leverage_vec);
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_user_settings(user_settings, nonce_val)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(signed.into())
    }

    // ========================================================================
    // Legacy methods (deprecated, kept for backward compatibility)
    // ========================================================================

    /// @deprecated Use sign() for single items, signAll() for batches, signGroup() for atomic
    #[napi]
    pub fn sign_order(
        &mut self,
        orders: Vec<OrderInput>,
        nonce: Option<f64>,
    ) -> Result<SignedTransactionOutput> {
        // Delegates to sign_group for backward compatibility
        self.sign_group(orders, nonce)
    }

    /// @deprecated Use signAll() instead
    #[napi]
    pub fn sign_orders_batch(
        &self,
        batches: Vec<Vec<OrderInput>>,
        base_nonce: Option<f64>,
    ) -> Result<Vec<SignedTransactionOutput>> {
        #[allow(deprecated)]
        {
            let order_batches: Result<Vec<Vec<OrderItem>>> = batches
                .into_iter()
                .map(|batch| batch.into_iter().map(|o| o.try_into()).collect())
                .collect();
            let order_batches = order_batches?;

            let base = base_nonce.map(|n| n as u64);
            let signed = self
                .inner
                .sign_orders_batch(order_batches, base)
                .map_err(|e| Error::from_reason(e.to_string()))?;

            Ok(signed.into_iter().map(Into::into).collect())
        }
    }
}

// ============================================================================
// Input/Output types for JS interop
// ============================================================================

#[napi(object)]
#[derive(Debug, Deserialize)]
pub struct OrderInput {
    #[napi(js_name = "type")]
    pub item_type: String,
    pub symbol: Option<String>,
    pub is_buy: Option<bool>,
    pub price: Option<f64>,
    pub size: Option<f64>,
    pub reduce_only: Option<bool>,
    pub order_type: Option<OrderTypeInput>,
    pub client_id: Option<String>,
    pub order_id: Option<String>,
    pub symbols: Option<Vec<String>>,
}

#[napi(object)]
#[derive(Debug, Deserialize)]
pub struct OrderTypeInput {
    #[napi(js_name = "type")]
    pub type_name: String,
    pub tif: Option<String>,
    pub is_market: Option<bool>,
    pub trigger_px: Option<f64>,
}

#[napi(object)]
#[derive(Debug)]
pub struct LeverageSetting {
    pub symbol: String,
    pub leverage: f64,
}

#[napi(object)]
#[derive(Debug)]
pub struct SignedTransactionOutput {
    /// Action JSON as string
    pub action: String,
    /// Account public key (base58)
    pub account: String,
    /// Signer public key (base58)
    pub signer: String,
    /// Signature (base58)
    pub signature: String,
    /// Pre-computed order/transaction ID (base58)
    /// This is SHA256(wincode_bytes), matching BULK's server-side ID generation.
    /// Available before server response for optimistic tracking.
    pub order_id: Option<String>,
}

impl From<bulk_keychain::SignedTransaction> for SignedTransactionOutput {
    fn from(tx: bulk_keychain::SignedTransaction) -> Self {
        Self {
            action: serde_json::to_string(&tx.action).unwrap_or_default(),
            account: tx.account,
            signer: tx.signer,
            signature: tx.signature,
            order_id: tx.order_id,
        }
    }
}

impl TryFrom<OrderInput> for OrderItem {
    type Error = Error;

    fn try_from(input: OrderInput) -> Result<Self> {
        match input.item_type.as_str() {
            "order" => {
                let symbol = input.symbol.ok_or_else(|| Error::from_reason("order.symbol is required"))?;
                let is_buy = input.is_buy.ok_or_else(|| Error::from_reason("order.isBuy is required"))?;
                let price = input.price.ok_or_else(|| Error::from_reason("order.price is required"))?;
                let size = input.size.ok_or_else(|| Error::from_reason("order.size is required"))?;
                let reduce_only = input.reduce_only.unwrap_or(false);

                let order_type = match input.order_type {
                    Some(ot) => match ot.type_name.as_str() {
                        "limit" => {
                            let tif_str = ot.tif.as_deref().unwrap_or("GTC");
                            let tif = match tif_str.to_uppercase().as_str() {
                                "GTC" => TimeInForce::Gtc,
                                "IOC" => TimeInForce::Ioc,
                                "ALO" => TimeInForce::Alo,
                                _ => return Err(Error::from_reason(format!("Invalid tif: {}", tif_str))),
                            };
                            OrderType::limit(tif)
                        }
                        "trigger" | "market" => OrderType::Trigger {
                            is_market: ot.is_market.unwrap_or(true),
                            trigger_px: ot.trigger_px.unwrap_or(0.0),
                        },
                        _ => return Err(Error::from_reason(format!("Invalid orderType: {}", ot.type_name))),
                    },
                    None => OrderType::limit(TimeInForce::Gtc),
                };

                let client_id = input
                    .client_id
                    .map(|s| Hash::from_base58(&s))
                    .transpose()
                    .map_err(|e| Error::from_reason(format!("Invalid clientId: {}", e)))?;

                let mut order = Order {
                    symbol,
                    is_buy,
                    price,
                    size,
                    reduce_only,
                    order_type,
                    client_id: None,
                };
                if let Some(cid) = client_id {
                    order.client_id = Some(cid);
                }

                Ok(OrderItem::Order(order))
            }
            "cancel" => {
                let symbol = input.symbol.ok_or_else(|| Error::from_reason("cancel.symbol is required"))?;
                let order_id_str = input.order_id.ok_or_else(|| Error::from_reason("cancel.orderId is required"))?;
                let order_id = Hash::from_base58(&order_id_str)
                    .map_err(|e| Error::from_reason(format!("Invalid orderId: {}", e)))?;

                Ok(OrderItem::Cancel(Cancel::new(symbol, order_id)))
            }
            "cancelAll" => {
                let symbols = input.symbols.unwrap_or_default();
                Ok(OrderItem::CancelAll(CancelAll::for_symbols(symbols)))
            }
            _ => Err(Error::from_reason(format!("Invalid item type: {}", input.item_type))),
        }
    }
}

// ============================================================================
// Utility functions
// ============================================================================

/// Generate a random hash (for client order IDs)
#[napi]
pub fn random_hash() -> String {
    Hash::random().to_base58()
}

/// Get current timestamp in milliseconds
#[napi]
pub fn current_timestamp() -> f64 {
    bulk_keychain::nonce::current_timestamp_millis() as f64
}

/// Validate a base58-encoded public key
#[napi]
pub fn validate_pubkey(s: String) -> bool {
    Pubkey::from_base58(&s).is_ok()
}

/// Validate a base58-encoded hash
#[napi]
pub fn validate_hash(s: String) -> bool {
    Hash::from_base58(&s).is_ok()
}

/// Compute order ID from wincode bytes
///
/// This computes SHA256(wincode_bytes), which matches BULK's server-side
/// order ID generation. Useful if you're serializing transactions yourself.
#[napi]
pub fn compute_order_id(wincode_bytes: Buffer) -> String {
    Hash::from_wincode_bytes(&wincode_bytes).to_base58()
}

// ============================================================================
// External Wallet Support - Prepare/Finalize API
// ============================================================================

/// Options for preparing a message
#[napi(object)]
#[derive(Debug)]
pub struct PrepareOptions {
    /// Account public key (base58) - the trading account
    pub account: String,
    /// Signer public key (base58) - defaults to account if not provided
    pub signer: Option<String>,
    /// Nonce - defaults to current timestamp if not provided
    pub nonce: Option<f64>,
}

/// Prepared message ready for external wallet signing
#[napi(object)]
pub struct PreparedMessageOutput {
    /// Raw message bytes to sign (pass to wallet.signMessage())
    pub message_bytes: Buffer,
    /// Message as base58 string
    pub message_base58: String,
    /// Message as base64 string
    pub message_base64: String,
    /// Message as hex string
    pub message_hex: String,
    /// Pre-computed order ID (base58)
    pub order_id: String,
    /// Action JSON as string
    pub action: String,
    /// Account public key (base58)
    pub account: String,
    /// Signer public key (base58)
    pub signer: String,
    /// Nonce used for this transaction
    pub nonce: f64,
}

impl From<PreparedMessage> for PreparedMessageOutput {
    fn from(p: PreparedMessage) -> Self {
        Self {
            message_bytes: Buffer::from(p.message_bytes.clone()),
            message_base58: p.message_base58(),
            message_base64: p.message_base64(),
            message_hex: p.message_hex(),
            order_id: p.order_id,
            action: serde_json::to_string(&p.action).unwrap_or_default(),
            account: p.account,
            signer: p.signer,
            nonce: p.nonce as f64,
        }
    }
}

/// Prepare a single order for external wallet signing
///
/// Use this when you don't have access to the private key and need
/// to sign with an external wallet (like Phantom, Privy, etc).
///
/// @example
/// ```typescript
/// const prepared = prepareOrder(order, { account: myPubkey });
/// const signature = await wallet.signMessage(prepared.messageBytes);
/// const signed = finalizeTransaction(prepared, signature);
/// ```
#[napi]
pub fn prepare_order(order: OrderInput, options: PrepareOptions) -> Result<PreparedMessageOutput> {
    let order_item: OrderItem = order.try_into()?;
    let account = Pubkey::from_base58(&options.account)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let signer = options
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let nonce = options.nonce.map(|n| n as u64);

    let prepared = prepare_message(order_item, &account, signer.as_ref(), nonce)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(prepared.into())
}

/// Prepare multiple orders - each becomes its own transaction (parallel)
///
/// Each order gets independent confirmation/rejection.
///
/// @example
/// ```typescript
/// const orders = [order1, order2, order3];
/// const prepared = prepareAllOrders(orders, { account: myPubkey });
/// // Sign each with wallet, then finalize
/// ```
#[napi]
pub fn prepare_all_orders(
    orders: Vec<OrderInput>,
    options: PrepareOptions,
) -> Result<Vec<PreparedMessageOutput>> {
    let order_items: Result<Vec<OrderItem>> = orders.into_iter().map(|o| o.try_into()).collect();
    let order_items = order_items?;

    let account = Pubkey::from_base58(&options.account)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let signer = options
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let base_nonce = options.nonce.map(|n| n as u64);

    let prepared = prepare_all(order_items, &account, signer.as_ref(), base_nonce)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(prepared.into_iter().map(Into::into).collect())
}

/// Prepare multiple orders as ONE atomic transaction
///
/// Use for bracket orders (entry + stop loss + take profit).
///
/// @example
/// ```typescript
/// const bracket = [entryOrder, stopLoss, takeProfit];
/// const prepared = prepareOrderGroup(bracket, { account: myPubkey });
/// const signature = await wallet.signMessage(prepared.messageBytes);
/// const signed = finalizeTransaction(prepared, signature);
/// ```
#[napi]
pub fn prepare_order_group(
    orders: Vec<OrderInput>,
    options: PrepareOptions,
) -> Result<PreparedMessageOutput> {
    let order_items: Result<Vec<OrderItem>> = orders.into_iter().map(|o| o.try_into()).collect();
    let order_items = order_items?;

    let account = Pubkey::from_base58(&options.account)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let signer = options
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let nonce = options.nonce.map(|n| n as u64);

    let prepared = prepare_group(order_items, &account, signer.as_ref(), nonce)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(prepared.into())
}

/// Prepare agent wallet creation for external signing
///
/// @example
/// ```typescript
/// const prepared = prepareAgentWalletAuth(agentPubkey, false, { account: myPubkey });
/// const signature = await wallet.signMessage(prepared.messageBytes);
/// const signed = finalizeTransaction(prepared, signature);
/// ```
#[napi]
pub fn prepare_agent_wallet_auth(
    agent_pubkey: String,
    delete: bool,
    options: PrepareOptions,
) -> Result<PreparedMessageOutput> {
    let agent = Pubkey::from_base58(&agent_pubkey)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let account = Pubkey::from_base58(&options.account)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let signer = options
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let nonce = options.nonce.map(|n| n as u64);

    let prepared = prepare_agent_wallet(&agent, delete, &account, signer.as_ref(), nonce)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(prepared.into())
}

/// Prepare faucet request for external signing
#[napi]
pub fn prepare_faucet_request(options: PrepareOptions) -> Result<PreparedMessageOutput> {
    let account = Pubkey::from_base58(&options.account)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let signer = options
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let nonce = options.nonce.map(|n| n as u64);

    let prepared = prepare_faucet(&account, signer.as_ref(), nonce)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(prepared.into())
}

/// Finalize a prepared message with a signature from an external wallet
///
/// @param prepared - The prepared message from prepare* functions
/// @param signature - Base58-encoded signature from wallet.signMessage()
///
/// @example
/// ```typescript
/// const prepared = prepareOrder(order, { account: myPubkey });
/// const signature = await wallet.signMessage(prepared.messageBytes);
/// const signed = finalizeTransaction(prepared, signature);
/// // Now submit `signed` to the API
/// ```
#[napi]
pub fn finalize_prepared_transaction(
    prepared: PreparedMessageOutput,
    signature: String,
) -> SignedTransactionOutput {
    // Reconstruct the PreparedMessage (we only need the fields for finalization)
    let action: serde_json::Value = serde_json::from_str(&prepared.action).unwrap_or_default();
    let signed = bulk_keychain::SignedTransaction {
        action,
        account: prepared.account,
        signer: prepared.signer,
        signature,
        order_id: Some(prepared.order_id),
    };
    signed.into()
}
