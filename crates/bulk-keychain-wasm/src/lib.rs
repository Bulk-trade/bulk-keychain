//! WASM bindings for BULK transaction signing
//!
//! This crate provides WebAssembly bindings for the bulk-keychain signing library,
//! enabling high-performance transaction signing in browser environments.

use bulk_keychain::{
    finalize_transaction, prepare_agent_wallet, prepare_all, prepare_create_sub_account,
    prepare_faucet, prepare_group, prepare_message, prepare_remove_sub_account, prepare_transfer,
    prepare_user_settings, Cancel, CancelAll, CreateSubAccount, Hash, Keypair, Modify,
    NonceManager, NonceStrategy, OnFill, OraclePrice, Order, OrderItem, OrderType, PreparedMessage,
    Pubkey, PythOraclePrice, RangeOco, Signer, Stop, TakeProfit, TimeInForce, TrailingStop,
    Transfer, TransferKind, TriggerBasket, UserSettings,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

// Initialize panic hook for better error messages in development
#[cfg(feature = "console_error_panic_hook")]
fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

#[cfg(not(feature = "console_error_panic_hook"))]
fn set_panic_hook() {}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    set_panic_hook();
}

// ============================================================================
// Keypair
// ============================================================================

/// WASM wrapper for Keypair
#[wasm_bindgen]
pub struct WasmKeypair {
    inner: Keypair,
}

#[wasm_bindgen]
impl WasmKeypair {
    /// Generate a new random keypair
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Keypair::generate(),
        }
    }

    /// Create from base58-encoded secret key or full keypair
    #[wasm_bindgen(js_name = fromBase58)]
    pub fn from_base58(s: &str) -> Result<WasmKeypair, JsError> {
        let inner = Keypair::from_base58(s).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create from raw bytes (32-byte secret or 64-byte full keypair)
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: &[u8]) -> Result<WasmKeypair, JsError> {
        let inner = Keypair::from_bytes(bytes).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { inner })
    }

    /// Get the public key as base58 string
    #[wasm_bindgen(getter)]
    pub fn pubkey(&self) -> String {
        self.inner.pubkey().to_base58()
    }

    /// Get the full keypair as base58 (64 bytes)
    #[wasm_bindgen(js_name = toBase58)]
    pub fn to_base58(&self) -> String {
        self.inner.to_base58()
    }

    /// Get the full keypair as bytes (64 bytes)
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes().to_vec()
    }

    /// Get the secret key as bytes (32 bytes)
    #[wasm_bindgen(js_name = secretKey)]
    pub fn secret_key(&self) -> Vec<u8> {
        self.inner.secret_key().to_vec()
    }
}

impl Default for WasmKeypair {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Signer
// ============================================================================

/// WASM wrapper for Signer
#[wasm_bindgen]
pub struct WasmSigner {
    inner: Signer,
}

#[wasm_bindgen]
impl WasmSigner {
    /// Create a new signer from a keypair
    #[wasm_bindgen(constructor)]
    pub fn new(keypair: &WasmKeypair) -> Self {
        Self {
            inner: Signer::new(keypair.inner.clone()),
        }
    }

    /// Create a signer from base58-encoded secret key
    #[wasm_bindgen(js_name = fromBase58)]
    pub fn from_base58(s: &str) -> Result<WasmSigner, JsError> {
        let keypair = Keypair::from_base58(s).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self {
            inner: Signer::new(keypair),
        })
    }

    /// Create a signer with nonce management
    #[wasm_bindgen(js_name = withNonceManager)]
    pub fn with_nonce_manager(
        keypair: &WasmKeypair,
        strategy: &str,
    ) -> Result<WasmSigner, JsError> {
        let nonce_strategy = match strategy {
            "timestamp" => NonceStrategy::Timestamp,
            "counter" => NonceStrategy::Counter,
            "highFrequency" => NonceStrategy::TimestampWithCounter,
            _ => {
                return Err(JsError::new(
                    "Invalid nonce strategy. Use 'timestamp', 'counter', or 'highFrequency'",
                ))
            }
        };
        let nonce_manager = NonceManager::new(nonce_strategy);
        Ok(Self {
            inner: Signer::with_nonce_manager(keypair.inner.clone(), nonce_manager),
        })
    }

    /// Get the signer's public key
    #[wasm_bindgen(getter)]
    pub fn pubkey(&self) -> String {
        self.inner.pubkey().to_base58()
    }

    /// Enable/disable single-order ID computation.
    #[wasm_bindgen(js_name = setComputeOrderId)]
    pub fn set_compute_order_id(&mut self, enabled: bool) {
        self.inner.set_order_id(enabled);
    }

    /// Enable/disable batch order ID computation for multi-order transactions.
    #[wasm_bindgen(js_name = setComputeBatchOrderIds)]
    pub fn set_compute_batch_order_ids(&mut self, enabled: bool) {
        self.inner.set_batch_order_ids(enabled);
    }

    /// Whether single-order ID computation is enabled.
    #[wasm_bindgen(js_name = computesOrderId)]
    pub fn computes_order_id(&self) -> bool {
        self.inner.computes_order_id()
    }

    /// Whether batch order ID computation is enabled.
    #[wasm_bindgen(js_name = computesBatchOrderIds)]
    pub fn computes_batch_order_ids(&self) -> bool {
        self.inner.computes_batch_order_ids()
    }

    // ========================================================================
    // Simplified API
    // ========================================================================

    /// Sign a single order/cancel/cancelAll
    #[wasm_bindgen]
    pub fn sign(&mut self, order: JsValue, nonce: Option<f64>) -> Result<JsValue, JsError> {
        let order_input: OrderInput =
            serde_wasm_bindgen::from_value(order).map_err(|e| JsError::new(&e.to_string()))?;

        let order_item: OrderItem = order_input
            .try_into()
            .map_err(|e: String| JsError::new(&e))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign(order_item, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign multiple orders - each becomes its own transaction (parallel)
    #[wasm_bindgen(js_name = signAll)]
    pub fn sign_all(&self, orders: JsValue, base_nonce: Option<f64>) -> Result<JsValue, JsError> {
        let order_inputs: Vec<OrderInput> =
            serde_wasm_bindgen::from_value(orders).map_err(|e| JsError::new(&e.to_string()))?;

        let order_items: Result<Vec<OrderItem>, _> =
            order_inputs.into_iter().map(|o| o.try_into()).collect();
        let order_items = order_items.map_err(|e: String| JsError::new(&e))?;

        let base = base_nonce.map(|n| n as u64);
        let signed = self
            .inner
            .sign_all(order_items, base)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign multiple orders atomically in ONE transaction
    #[wasm_bindgen(js_name = signGroup)]
    pub fn sign_group(&mut self, orders: JsValue, nonce: Option<f64>) -> Result<JsValue, JsError> {
        let order_inputs: Vec<OrderInput> =
            serde_wasm_bindgen::from_value(orders).map_err(|e| JsError::new(&e.to_string()))?;

        let order_items: Result<Vec<OrderItem>, _> =
            order_inputs.into_iter().map(|o| o.try_into()).collect();
        let order_items = order_items.map_err(|e: String| JsError::new(&e))?;

        let nonce_val = nonce.map(|n| n as u64);
        let signed = self
            .inner
            .sign_group(order_items, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    // ========================================================================
    // Other signing methods
    // ========================================================================

    /// Sign a faucet request (testnet only)
    #[wasm_bindgen(js_name = signFaucet)]
    pub fn sign_faucet(&mut self, nonce: Option<f64>) -> Result<JsValue, JsError> {
        let nonce_val = nonce.map(|n| n as u64);
        let signed = self
            .inner
            .sign_faucet(nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign agent wallet creation/deletion
    #[wasm_bindgen(js_name = signAgentWallet)]
    pub fn sign_agent_wallet(
        &mut self,
        agent_pubkey: &str,
        delete: bool,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let agent = Pubkey::from_base58(agent_pubkey).map_err(|e| JsError::new(&e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_agent_wallet(agent, delete, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign user settings update
    #[wasm_bindgen(js_name = signUserSettings)]
    pub fn sign_user_settings(
        &mut self,
        settings: JsValue,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let settings_input: UserSettingsInput =
            serde_wasm_bindgen::from_value(settings).map_err(|e| JsError::new(&e.to_string()))?;

        let user_settings = UserSettings::new(settings_input.max_leverage);
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_user_settings(user_settings, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign one or more oracle price updates (`px`)
    #[wasm_bindgen(js_name = signOraclePrices)]
    pub fn sign_oracle_prices(
        &mut self,
        oracles: JsValue,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let oracle_inputs: Vec<OraclePriceInput> =
            serde_wasm_bindgen::from_value(oracles).map_err(|e| JsError::new(&e.to_string()))?;
        let oracle_prices: Vec<OraclePrice> = oracle_inputs
            .into_iter()
            .map(|o| OraclePrice {
                timestamp: o.timestamp,
                asset: o.asset,
                price: o.price,
            })
            .collect();
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_oracle_prices(oracle_prices, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a batch Pyth oracle update (`o`)
    #[wasm_bindgen(js_name = signPythOracle)]
    pub fn sign_pyth_oracle(
        &mut self,
        oracles: JsValue,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let oracle_inputs: Vec<PythOraclePriceInput> =
            serde_wasm_bindgen::from_value(oracles).map_err(|e| JsError::new(&e.to_string()))?;
        let pyth_oracles: Vec<PythOraclePrice> = oracle_inputs
            .into_iter()
            .map(|o| PythOraclePrice {
                timestamp: o.timestamp,
                feed_index: o.feed_index,
                price: o.price,
                exponent: o.exponent,
            })
            .collect();
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_pyth_oracle(pyth_oracles, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a margin transfer between accounts
    ///
    /// @param kind - "internal" or "external" (defaults to "internal")
    /// @param fromPubkey - source account pubkey (base58)
    /// @param toPubkey - destination account pubkey (base58)
    /// @param marginSymbol - margin asset symbol (e.g. "USDC")
    /// @param marginAmount - amount to transfer
    /// @param nonce - optional nonce
    #[wasm_bindgen(js_name = signTransfer)]
    pub fn sign_transfer(
        &mut self,
        kind: Option<String>,
        from_pubkey: &str,
        to_pubkey: &str,
        margin_symbol: String,
        margin_amount: f64,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let from = Pubkey::from_base58(from_pubkey).map_err(|e| JsError::new(&e.to_string()))?;
        let to = Pubkey::from_base58(to_pubkey).map_err(|e| JsError::new(&e.to_string()))?;
        let kind = match kind.as_deref() {
            Some("external") => TransferKind::External,
            Some("internal") | None => TransferKind::Internal,
            Some(other) => return Err(JsError::new(&format!("Invalid transfer kind: {}", other))),
        };
        let nonce_val = nonce.map(|n| n as u64);
        let transfer = Transfer {
            kind,
            from,
            to,
            margin_symbol,
            margin_amount,
        };

        let signed = self
            .inner
            .sign_transfer(transfer, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a sub-account creation (optional initial margin transfer)
    #[wasm_bindgen(js_name = signCreateSubAccount)]
    pub fn sign_create_sub_account(
        &mut self,
        name: String,
        margin_symbol: Option<String>,
        margin_amount: Option<f64>,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let nonce_val = nonce.map(|n| n as u64);
        let sub_account = CreateSubAccount {
            name,
            margin_symbol,
            margin_amount,
        };

        let signed = self
            .inner
            .sign_create_sub_account(sub_account, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a sub-account removal
    ///
    /// @param toRemove - sub-account pubkey to remove (base58)
    /// @param nonce - optional nonce
    #[wasm_bindgen(js_name = signRemoveSubAccount)]
    pub fn sign_remove_sub_account(
        &mut self,
        to_remove: &str,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let target = Pubkey::from_base58(to_remove).map_err(|e| JsError::new(&e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_remove_sub_account(target, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign whitelist/un-whitelist faucet access (`whitelistFaucet`)
    #[wasm_bindgen(js_name = signWhitelistFaucet)]
    pub fn sign_whitelist_faucet(
        &mut self,
        target_pubkey: &str,
        whitelist: bool,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let target =
            Pubkey::from_base58(target_pubkey).map_err(|e| JsError::new(&e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_whitelist_faucet(target, whitelist, nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    // ========================================================================
    // Legacy methods (deprecated, kept for backward compatibility)
    // ========================================================================

    /// @deprecated Use sign(), signAll(), or signGroup() instead
    #[wasm_bindgen(js_name = signOrder)]
    pub fn sign_order(&mut self, orders: JsValue, nonce: Option<f64>) -> Result<JsValue, JsError> {
        self.sign_group(orders, nonce)
    }

    /// @deprecated Use signAll() instead
    #[wasm_bindgen(js_name = signOrdersBatch)]
    pub fn sign_orders_batch(
        &self,
        batches: JsValue,
        base_nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        #[allow(deprecated)]
        {
            let batch_inputs: Vec<Vec<OrderInput>> = serde_wasm_bindgen::from_value(batches)
                .map_err(|e| JsError::new(&e.to_string()))?;

            let order_batches: Result<Vec<Vec<OrderItem>>, String> = batch_inputs
                .into_iter()
                .map(|batch| batch.into_iter().map(|o| o.try_into()).collect())
                .collect();
            let order_batches = order_batches.map_err(|e| JsError::new(&e))?;

            let base = base_nonce.map(|n| n as u64);
            let signed = self
                .inner
                .sign_orders_batch(order_batches, base)
                .map_err(|e| JsError::new(&e.to_string()))?;

            serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
        }
    }
}

// ============================================================================
// Input types for JS interop
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnFillInput {
    p: u32,
    actions: Vec<OrderInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderInput {
    #[serde(rename = "type")]
    item_type: String,
    symbol: Option<String>,
    is_buy: Option<bool>,
    price: Option<f64>,
    size: Option<f64>,
    reduce_only: Option<bool>,
    iso: Option<bool>,
    order_type: Option<OrderTypeInput>,
    client_id: Option<String>,
    order_id: Option<String>,
    amount: Option<f64>,
    symbols: Option<Vec<String>>,
    trigger_price: Option<f64>,
    limit_price: Option<f64>,
    pmin: Option<f64>,
    pmax: Option<f64>,
    lmin: Option<f64>,
    lmax: Option<f64>,
    actions: Option<Vec<OrderInput>>,
    on_fill: Option<OnFillInput>,
    trail_bps: Option<u32>,
    step_bps: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderTypeInput {
    #[serde(rename = "type")]
    type_name: String,
    tif: Option<String>,
    is_market: Option<bool>,
    trigger_px: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserSettingsInput {
    max_leverage: Vec<(String, f64)>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OraclePriceInput {
    timestamp: u64,
    asset: String,
    price: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PythOraclePriceInput {
    timestamp: u64,
    feed_index: u64,
    price: u64,
    exponent: i16,
}

impl TryFrom<OrderInput> for OrderItem {
    type Error = String;

    fn try_from(input: OrderInput) -> Result<Self, Self::Error> {
        match input.item_type.as_str() {
            "order" => {
                let symbol = input.symbol.ok_or("order.symbol is required")?;
                let is_buy = input.is_buy.ok_or("order.isBuy is required")?;
                let price = input.price.ok_or("order.price is required")?;
                let size = input.size.ok_or("order.size is required")?;
                let reduce_only = input.reduce_only.unwrap_or(false);
                let iso = input.iso.unwrap_or(false);

                let order_type = match input.order_type {
                    Some(ot) => match ot.type_name.as_str() {
                        "limit" => {
                            let tif_str = ot.tif.as_deref().unwrap_or("GTC");
                            let tif = match tif_str.to_uppercase().as_str() {
                                "GTC" => TimeInForce::Gtc,
                                "IOC" => TimeInForce::Ioc,
                                "ALO" => TimeInForce::Alo,
                                _ => return Err(format!("Invalid tif: {}", tif_str)),
                            };
                            OrderType::limit(tif)
                        }
                        "trigger" | "market" => OrderType::Trigger {
                            is_market: ot.is_market.unwrap_or(true),
                            trigger_px: ot.trigger_px.unwrap_or(0.0),
                        },
                        _ => return Err(format!("Invalid orderType: {}", ot.type_name)),
                    },
                    None => OrderType::limit(TimeInForce::Gtc),
                };

                let client_id = input
                    .client_id
                    .map(|s| Hash::from_base58(&s))
                    .transpose()
                    .map_err(|e| format!("Invalid clientId: {}", e))?;

                let mut order = Order {
                    symbol,
                    is_buy,
                    price,
                    size,
                    reduce_only,
                    iso,
                    order_type,
                    client_id: None,
                };
                if let Some(cid) = client_id {
                    order.client_id = Some(cid);
                }

                Ok(OrderItem::Order(order))
            }
            "cancel" => {
                let symbol = input.symbol.ok_or("cancel.symbol is required")?;
                let order_id_str = input.order_id.ok_or("cancel.orderId is required")?;
                let order_id = Hash::from_base58(&order_id_str)
                    .map_err(|e| format!("Invalid orderId: {}", e))?;

                Ok(OrderItem::Cancel(Cancel::new(symbol, order_id)))
            }
            "modify" => {
                let symbol = input.symbol.ok_or("modify.symbol is required")?;
                let order_id_str = input.order_id.ok_or("modify.orderId is required")?;
                let amount = input.amount.ok_or("modify.amount is required")?;
                let order_id = Hash::from_base58(&order_id_str)
                    .map_err(|e| format!("Invalid orderId: {}", e))?;
                Ok(OrderItem::Modify(Modify::new(order_id, symbol, amount)))
            }
            "cancelAll" => {
                let symbols = input.symbols.unwrap_or_default();
                Ok(OrderItem::CancelAll(CancelAll::for_symbols(symbols)))
            }
            "stop" | "st" => {
                let symbol = input.symbol.ok_or("stop.symbol is required")?;
                let is_buy = input.is_buy.ok_or("stop.isBuy is required")?;
                let size = input.size.ok_or("stop.size is required")?;
                let trigger_price = input.trigger_price.ok_or("stop.triggerPrice is required")?;
                let limit_price = input.limit_price.unwrap_or(f64::NAN);
                Ok(OrderItem::Stop(Stop {
                    symbol,
                    is_buy,
                    size,
                    trigger_price,
                    limit_price,
                    iso: input.iso.unwrap_or(false),
                }))
            }
            "takeProfit" | "tp" => {
                let symbol = input.symbol.ok_or("takeProfit.symbol is required")?;
                let is_buy = input.is_buy.ok_or("takeProfit.isBuy is required")?;
                let size = input.size.ok_or("takeProfit.size is required")?;
                let trigger_price = input
                    .trigger_price
                    .ok_or("takeProfit.triggerPrice is required")?;
                let limit_price = input.limit_price.unwrap_or(f64::NAN);
                Ok(OrderItem::TakeProfit(TakeProfit {
                    symbol,
                    is_buy,
                    size,
                    trigger_price,
                    limit_price,
                    iso: input.iso.unwrap_or(false),
                }))
            }
            "range" | "rng" => {
                let symbol = input.symbol.ok_or("range.symbol is required")?;
                let is_buy = input.is_buy.ok_or("range.isBuy is required")?;
                let size = input.size.ok_or("range.size is required")?;
                let collar_min = input.pmin.ok_or("range.pmin is required")?;
                let collar_max = input.pmax.ok_or("range.pmax is required")?;
                let limit_min = input.lmin.unwrap_or(f64::NAN);
                let limit_max = input.lmax.unwrap_or(f64::NAN);
                Ok(OrderItem::RangeOco(RangeOco {
                    symbol,
                    is_buy,
                    size,
                    collar_min,
                    collar_max,
                    limit_min,
                    limit_max,
                    iso: input.iso.unwrap_or(false),
                }))
            }
            "trig" => {
                let symbol = input.symbol.ok_or("trig.symbol is required")?;
                let is_buy = input.is_buy.ok_or("trig.isBuy is required")?;
                let trigger_price = input.trigger_price.ok_or("trig.triggerPrice is required")?;
                let raw_actions = input.actions.ok_or("trig.actions is required")?;
                let iso = input.iso.unwrap_or(false);
                let actions: Result<Vec<OrderItem>, String> =
                    raw_actions.into_iter().map(|a| a.try_into()).collect();
                Ok(OrderItem::TriggerBasket(TriggerBasket {
                    symbol,
                    is_buy,
                    trigger_price,
                    actions: actions?,
                    iso,
                }))
            }
            "onFill" | "of" => {
                let raw_actions = input.actions.ok_or("onFill.actions is required")?;
                let actions: Result<Vec<OrderItem>, String> =
                    raw_actions.into_iter().map(|a| a.try_into()).collect();
                Ok(OrderItem::OnFill(OnFill {
                    p: 0,
                    actions: actions?,
                }))
            }
            "trailingStop" | "trl" => {
                let symbol = input.symbol.ok_or("trl.symbol is required")?;
                let is_buy = input.is_buy.ok_or("trl.isBuy is required")?;
                let size = input.size.ok_or("trl.size is required")?;
                let trail_bps = input.trail_bps.ok_or("trl.trailBps is required")?;
                let step_bps = input.step_bps.ok_or("trl.stepBps is required")?;
                let limit_price = input.limit_price;
                Ok(OrderItem::TrailingStop(TrailingStop {
                    symbol,
                    is_buy,
                    size,
                    trail_bps,
                    step_bps,
                    limit_price,
                    iso: input.iso.unwrap_or(false),
                }))
            }
            _ => Err(format!("Invalid item type: {}", input.item_type)),
        }
    }
}

// ============================================================================
// Utility functions
// ============================================================================

/// Generate a random hash (for client order IDs)
#[wasm_bindgen(js_name = randomHash)]
pub fn random_hash() -> String {
    Hash::random().to_base58()
}

/// Get current timestamp in milliseconds
#[wasm_bindgen(js_name = currentTimestamp)]
pub fn current_timestamp() -> f64 {
    bulk_keychain::nonce::current_timestamp_millis() as f64
}

/// Validate a base58-encoded public key
#[wasm_bindgen(js_name = validatePubkey)]
pub fn validate_pubkey(s: &str) -> bool {
    Pubkey::from_base58(s).is_ok()
}

/// Validate a base58-encoded hash
#[wasm_bindgen(js_name = validateHash)]
pub fn validate_hash(s: &str) -> bool {
    Hash::from_base58(s).is_ok()
}

/// Compute SHA256 hash from raw bytes.
///
/// This is a raw utility and does not apply BULK order-ID canonicalization.
#[wasm_bindgen(js_name = computeOrderId)]
pub fn compute_order_id(wincode_bytes: &[u8]) -> String {
    Hash::from_wincode_bytes(wincode_bytes).to_base58()
}

// ============================================================================
// External Wallet Support - Prepare/Finalize API
// ============================================================================

/// Prepared message for external wallet signing
///
/// This contains everything needed to sign with an external wallet
/// and then finalize into a SignedTransaction.
#[wasm_bindgen]
pub struct WasmPreparedMessage {
    inner: PreparedMessage,
}

#[wasm_bindgen]
impl WasmPreparedMessage {
    /// Get the raw message bytes to sign (Uint8Array)
    #[wasm_bindgen(getter, js_name = messageBytes)]
    pub fn message_bytes(&self) -> Vec<u8> {
        self.inner.message_bytes.clone()
    }

    /// Get message as base58 string
    #[wasm_bindgen(getter, js_name = messageBase58)]
    pub fn message_base58(&self) -> String {
        self.inner.message_base58()
    }

    /// Get message as base64 string
    #[wasm_bindgen(getter, js_name = messageBase64)]
    pub fn message_base64(&self) -> String {
        self.inner.message_base64()
    }

    /// Get message as hex string
    #[wasm_bindgen(getter, js_name = messageHex)]
    pub fn message_hex(&self) -> String {
        self.inner.message_hex()
    }

    /// Get the pre-computed order ID (single-order tx only)
    #[wasm_bindgen(getter, js_name = orderId)]
    pub fn order_id(&self) -> Option<String> {
        self.inner.order_id.clone()
    }

    /// Get pre-computed order IDs (multi-order tx only)
    #[wasm_bindgen(getter, js_name = orderIds)]
    pub fn order_ids(&self) -> Option<Vec<String>> {
        self.inner.order_ids.clone()
    }

    /// Get the actions JSON
    #[wasm_bindgen(getter)]
    pub fn actions(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.inner.actions).unwrap_or(JsValue::NULL)
    }

    /// Get the account public key (base58)
    #[wasm_bindgen(getter)]
    pub fn account(&self) -> String {
        self.inner.account.clone()
    }

    /// Get the signer public key (base58)
    #[wasm_bindgen(getter)]
    pub fn signer(&self) -> String {
        self.inner.signer.clone()
    }

    /// Get the nonce
    #[wasm_bindgen(getter)]
    pub fn nonce(&self) -> f64 {
        self.inner.nonce as f64
    }

    /// Finalize with a signature (base58 string)
    ///
    /// Call this after your wallet signs the messageBytes.
    #[wasm_bindgen]
    pub fn finalize(&self, signature: &str) -> JsValue {
        let signed = finalize_transaction(self.inner.clone(), signature);
        serde_wasm_bindgen::to_value(&signed).unwrap_or(JsValue::NULL)
    }

    /// Finalize with signature bytes (Uint8Array)
    #[wasm_bindgen(js_name = finalizeBytes)]
    pub fn finalize_bytes(&self, signature: &[u8]) -> JsValue {
        let sig_b58 = bulk_keychain::bs58::encode(signature).into_string();
        self.finalize(&sig_b58)
    }
}

/// Options for preparing a message
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrepareOptions {
    /// Account public key (base58) - the trading account
    account: String,
    /// Signer public key (base58) - defaults to account if not provided
    signer: Option<String>,
    /// Nonce - defaults to current timestamp if not provided
    nonce: Option<f64>,
}

/// Prepare a single order for external wallet signing
///
/// Use this when you don't have access to the private key and need
/// to sign with an external wallet (like Phantom, Privy, etc).
///
/// @param order - The order to prepare
/// @param options - { account: string, signer?: string, nonce?: number }
/// @returns PreparedMessage with messageBytes to sign
///
/// @example
/// ```typescript
/// const prepared = prepareOrder(order, { account: myPubkey });
/// const signature = await wallet.signMessage(prepared.messageBytes);
/// const signed = prepared.finalize(signature);
/// ```
#[wasm_bindgen(js_name = prepareOrder)]
pub fn wasm_prepare_order(
    order: JsValue,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let order_input: OrderInput =
        serde_wasm_bindgen::from_value(order).map_err(|e| JsError::new(&e.to_string()))?;
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    // If onFill is present, emit parent + OnFill as an atomic group
    let on_fill_input = order_input.on_fill;
    let order_input_no_fill = OrderInput {
        on_fill: None,
        ..order_input
    };

    let prepared = if let Some(of) = on_fill_input {
        let parent: OrderItem = order_input_no_fill
            .try_into()
            .map_err(|e: String| JsError::new(&e))?;
        let consequents: Result<Vec<OrderItem>, String> =
            of.actions.into_iter().map(|a| a.try_into()).collect();
        let of_item = OrderItem::OnFill(OnFill {
            p: of.p,
            actions: consequents.map_err(|e| JsError::new(&e))?,
        });
        prepare_group(vec![parent, of_item], &account, signer.as_ref(), nonce)
            .map_err(|e| JsError::new(&e.to_string()))?
    } else {
        let order_item: OrderItem = order_input_no_fill
            .try_into()
            .map_err(|e: String| JsError::new(&e))?;
        prepare_message(order_item, &account, signer.as_ref(), nonce)
            .map_err(|e| JsError::new(&e.to_string()))?
    };

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare multiple orders - each becomes its own transaction (parallel)
///
/// @param orders - Array of orders to prepare
/// @param options - { account: string, signer?: string, nonce?: number }
/// @returns Array of PreparedMessage
#[wasm_bindgen(js_name = prepareAll)]
pub fn wasm_prepare_all(
    orders: JsValue,
    options: JsValue,
) -> Result<Vec<WasmPreparedMessage>, JsError> {
    let order_inputs: Vec<OrderInput> =
        serde_wasm_bindgen::from_value(orders).map_err(|e| JsError::new(&e.to_string()))?;
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let order_items: Result<Vec<OrderItem>, String> =
        order_inputs.into_iter().map(|o| o.try_into()).collect();
    let order_items = order_items.map_err(|e| JsError::new(&e))?;

    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let base_nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_all(order_items, &account, signer.as_ref(), base_nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(prepared
        .into_iter()
        .map(|p| WasmPreparedMessage { inner: p })
        .collect())
}

/// Prepare multiple orders as ONE atomic transaction
///
/// Use for bracket orders (entry + stop loss + take profit).
///
/// @param orders - Array of orders for the atomic transaction
/// @param options - { account: string, signer?: string, nonce?: number }
/// @returns Single PreparedMessage containing all orders
#[wasm_bindgen(js_name = prepareGroup)]
pub fn wasm_prepare_group(
    orders: JsValue,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let order_inputs: Vec<OrderInput> =
        serde_wasm_bindgen::from_value(orders).map_err(|e| JsError::new(&e.to_string()))?;
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let order_items: Result<Vec<OrderItem>, String> =
        order_inputs.into_iter().map(|o| o.try_into()).collect();
    let order_items = order_items.map_err(|e| JsError::new(&e))?;

    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_group(order_items, &account, signer.as_ref(), nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare agent wallet creation for external signing
///
/// @param agentPubkey - The agent wallet public key to authorize
/// @param delete - Whether to delete (true) or add (false) the agent
/// @param options - { account: string, signer?: string, nonce?: number }
#[wasm_bindgen(js_name = prepareAgentWallet)]
pub fn wasm_prepare_agent_wallet(
    agent_pubkey: &str,
    delete: bool,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let agent = Pubkey::from_base58(agent_pubkey).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_agent_wallet(&agent, delete, &account, signer.as_ref(), nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare faucet request for external signing
///
/// @param options - { account: string, signer?: string, nonce?: number }
#[wasm_bindgen(js_name = prepareFaucet)]
pub fn wasm_prepare_faucet(options: JsValue) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_faucet(&account, signer.as_ref(), nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare user settings update for external signing
///
/// @param settings - { max_leverage: [[symbol, leverage], ...] }
/// @param options - { account: string, signer?: string, nonce?: number }
#[wasm_bindgen(js_name = prepareUpdateUserSettings)]
pub fn wasm_prepare_update_user_settings(
    settings: JsValue,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let settings_input: UserSettingsInput =
        serde_wasm_bindgen::from_value(settings).map_err(|e| JsError::new(&e.to_string()))?;
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let user_settings = UserSettings::new(settings_input.max_leverage);
    let prepared = prepare_user_settings(user_settings, &account, signer.as_ref(), nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a margin transfer for external signing
///
/// @param fromPubkey - source account pubkey (base58)
/// @param toPubkey - destination account pubkey (base58)
/// @param marginSymbol - margin asset symbol
/// @param marginAmount - amount to transfer
/// @param options - { account: string, signer?: string, nonce?: number, kind?: "internal" | "external" }
#[wasm_bindgen(js_name = prepareTransfer)]
pub fn wasm_prepare_transfer(
    from_pubkey: &str,
    to_pubkey: &str,
    margin_symbol: String,
    margin_amount: f64,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TransferOptions {
        account: String,
        #[serde(default)]
        signer: Option<String>,
        #[serde(default)]
        nonce: Option<f64>,
        #[serde(default)]
        kind: Option<String>,
    }

    let opts: TransferOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let from = Pubkey::from_base58(from_pubkey).map_err(|e| JsError::new(&e.to_string()))?;
    let to = Pubkey::from_base58(to_pubkey).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);
    let kind = match opts.kind.as_deref() {
        Some("external") => TransferKind::External,
        Some("internal") | None => TransferKind::Internal,
        Some(other) => return Err(JsError::new(&format!("Invalid transfer kind: {}", other))),
    };

    let transfer = Transfer {
        kind,
        from,
        to,
        margin_symbol,
        margin_amount,
    };

    let prepared = prepare_transfer(transfer, &account, signer.as_ref(), nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a sub-account removal for external signing
///
/// @param toRemove - sub-account pubkey to remove (base58)
/// @param options - { account: string, signer?: string, nonce?: number }
#[wasm_bindgen(js_name = prepareRemoveSubAccount)]
pub fn wasm_prepare_remove_sub_account(
    to_remove: &str,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let target = Pubkey::from_base58(to_remove).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_remove_sub_account(target, &account, signer.as_ref(), nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a sub-account creation for external signing
///
/// @param name - Sub-account display name
/// @param options - { account: string, signer?: string, nonce?: number, marginSymbol?: string, marginAmount?: number }
#[wasm_bindgen(js_name = prepareCreateSubAccount)]
pub fn wasm_prepare_create_sub_account(
    name: String,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CreateSubAccountOptions {
        account: String,
        #[serde(default)]
        signer: Option<String>,
        #[serde(default)]
        nonce: Option<f64>,
        #[serde(default)]
        margin_symbol: Option<String>,
        #[serde(default)]
        margin_amount: Option<f64>,
    }

    let opts: CreateSubAccountOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let sub_account = CreateSubAccount {
        name,
        margin_symbol: opts.margin_symbol,
        margin_amount: opts.margin_amount,
    };

    let prepared = prepare_create_sub_account(sub_account, &account, signer.as_ref(), nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Finalize a prepared message with a signature
///
/// Alternative to calling prepared.finalize() - useful if you have
/// the prepared message as a plain object.
#[wasm_bindgen(js_name = finalizeTransaction)]
pub fn wasm_finalize_transaction(prepared: JsValue, signature: &str) -> Result<JsValue, JsError> {
    let prep: PreparedMessage =
        serde_wasm_bindgen::from_value(prepared).map_err(|e| JsError::new(&e.to_string()))?;
    let signed = finalize_transaction(prep, signature);
    serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_keypair_generation() {
        let keypair = WasmKeypair::new();
        let pubkey = keypair.pubkey();
        assert!(!pubkey.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_keypair_roundtrip() {
        let keypair = WasmKeypair::new();
        let b58 = keypair.to_base58();
        let restored = WasmKeypair::from_base58(&b58).unwrap();
        assert_eq!(keypair.pubkey(), restored.pubkey());
    }
}
