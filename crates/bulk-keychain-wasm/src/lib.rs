//! WASM bindings for BULK transaction signing
//!
//! This crate provides WebAssembly bindings for the bulk-keychain signing library,
//! enabling high-performance transaction signing in browser environments.

use bulk_keychain::{
    Cancel, CancelAll, Hash, Keypair, NonceManager, NonceStrategy, Order, OrderItem,
    OrderType, Pubkey, Signer, TimeInForce, UserSettings,
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
    pub fn with_nonce_manager(keypair: &WasmKeypair, strategy: &str) -> Result<WasmSigner, JsError> {
        let nonce_strategy = match strategy {
            "timestamp" => NonceStrategy::Timestamp,
            "counter" => NonceStrategy::Counter,
            "highFrequency" => NonceStrategy::TimestampWithCounter,
            _ => return Err(JsError::new("Invalid nonce strategy. Use 'timestamp', 'counter', or 'highFrequency'")),
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

    // ========================================================================
    // Simplified API
    // ========================================================================

    /// Sign a single order/cancel/cancelAll
    #[wasm_bindgen]
    pub fn sign(&mut self, order: JsValue, nonce: Option<f64>) -> Result<JsValue, JsError> {
        let order_input: OrderInput =
            serde_wasm_bindgen::from_value(order).map_err(|e| JsError::new(&e.to_string()))?;

        let order_item: OrderItem = order_input.try_into().map_err(|e: String| JsError::new(&e))?;
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
            let batch_inputs: Vec<Vec<OrderInput>> =
                serde_wasm_bindgen::from_value(batches).map_err(|e| JsError::new(&e.to_string()))?;

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
struct OrderInput {
    #[serde(rename = "type")]
    item_type: String,
    symbol: Option<String>,
    is_buy: Option<bool>,
    price: Option<f64>,
    size: Option<f64>,
    reduce_only: Option<bool>,
    order_type: Option<OrderTypeInput>,
    client_id: Option<String>,
    order_id: Option<String>,
    symbols: Option<Vec<String>>,
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
            "cancelAll" => {
                let symbols = input.symbols.unwrap_or_default();
                Ok(OrderItem::CancelAll(CancelAll::for_symbols(symbols)))
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

/// Compute order ID from wincode bytes
///
/// This computes SHA256(wincode_bytes), which matches BULK's server-side
/// order ID generation. Useful if you're serializing transactions yourself.
#[wasm_bindgen(js_name = computeOrderId)]
pub fn compute_order_id(wincode_bytes: &[u8]) -> String {
    Hash::from_wincode_bytes(wincode_bytes).to_base58()
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
