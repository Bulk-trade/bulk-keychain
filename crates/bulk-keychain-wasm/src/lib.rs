//! WASM bindings for BULK transaction signing
//!
//! This crate provides WebAssembly bindings for the bulk-keychain signing library,
//! enabling high-performance transaction signing in browser environments.

use bulk_keychain::{
    finalize_transaction, prepare_agent_wallet, prepare_all, prepare_create_multisig,
    prepare_create_sub_account, prepare_faucet, prepare_group, prepare_message,
    prepare_multisig_approve, prepare_multisig_cancel, prepare_multisig_execute,
    prepare_multisig_propose, prepare_multisig_reject, prepare_remove_sub_account,
    prepare_rename_sub_account, prepare_transfer, prepare_update_multisig_policy,
    prepare_user_settings, Action, AgentWallet, Cancel, CancelAll, CreateMultisig,
    CreateSubAccount, Faucet, Hash, Keypair, Modify, MultisigApprove, MultisigCancel,
    MultisigExecute, MultisigPropose, MultisigReject, NonceManager, NonceStrategy, OnFill,
    OraclePrice, Order, OrderItem, OrderType, PreparedMessage, Pubkey, PythOraclePrice,
    RangeOco, RenameSubAccount, Signer, Stop, TakeProfit, TimeInForce, TrailingStop, Transfer,
    TransferKind, TriggerBasket, UpdateMultisigPolicy, UserSettings, WhitelistFaucet,
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
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

    /// Sign a multisig creation
    #[wasm_bindgen(js_name = signCreateMultisig)]
    pub fn sign_create_multisig(
        &mut self,
        signers: JsValue,
        threshold: u32,
        time_lock_secs: Option<u32>,
        proposal_lifetime_secs: Option<u32>,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let signer_inputs: Vec<String> =
            serde_wasm_bindgen::from_value(signers).map_err(|e| JsError::new(&e.to_string()))?;
        let signers = signer_inputs
            .into_iter()
            .map(|s| Pubkey::from_base58(&s).map_err(|e| JsError::new(&e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_create_multisig(
                CreateMultisig {
                    signers,
                    threshold,
                    time_lock_secs: time_lock_secs.unwrap_or(0),
                    proposal_lifetime_secs: proposal_lifetime_secs.unwrap_or(7 * 24 * 3600),
                },
                nonce_val,
            )
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a multisig proposal
    #[wasm_bindgen(js_name = signMultisigPropose)]
    pub fn sign_multisig_propose(
        &mut self,
        multisig: &str,
        actions: JsValue,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
        let actions = parse_action_values(actions)?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_multisig_propose(MultisigPropose::new(multisig, actions), nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a multisig approval
    #[wasm_bindgen(js_name = signMultisigApprove)]
    pub fn sign_multisig_approve(
        &mut self,
        multisig: &str,
        proposal_id: f64,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_multisig_approve(MultisigApprove::new(multisig, proposal_id as u64), nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a multisig rejection
    #[wasm_bindgen(js_name = signMultisigReject)]
    pub fn sign_multisig_reject(
        &mut self,
        multisig: &str,
        proposal_id: f64,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_multisig_reject(MultisigReject::new(multisig, proposal_id as u64), nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a multisig cancellation
    #[wasm_bindgen(js_name = signMultisigCancel)]
    pub fn sign_multisig_cancel(
        &mut self,
        multisig: &str,
        proposal_id: f64,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_multisig_cancel(MultisigCancel::new(multisig, proposal_id as u64), nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a multisig execution
    #[wasm_bindgen(js_name = signMultisigExecute)]
    pub fn sign_multisig_execute(
        &mut self,
        multisig: &str,
        proposal_id: f64,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_multisig_execute(MultisigExecute::new(multisig, proposal_id as u64), nonce_val)
            .map_err(|e| JsError::new(&e.to_string()))?;

        serde_wasm_bindgen::to_value(&signed).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Sign a multisig policy update
    #[wasm_bindgen(js_name = signUpdateMultisigPolicy)]
    pub fn sign_update_multisig_policy(
        &mut self,
        multisig: &str,
        signers: JsValue,
        threshold: u32,
        time_lock_secs: Option<u32>,
        proposal_lifetime_secs: Option<u32>,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
        let signer_inputs: Vec<String> =
            serde_wasm_bindgen::from_value(signers).map_err(|e| JsError::new(&e.to_string()))?;
        let signers = signer_inputs
            .into_iter()
            .map(|s| Pubkey::from_base58(&s).map_err(|e| JsError::new(&e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_update_multisig_policy(
                UpdateMultisigPolicy {
                    multisig,
                    signers,
                    threshold,
                    time_lock_secs: time_lock_secs.unwrap_or(0),
                    proposal_lifetime_secs: proposal_lifetime_secs.unwrap_or(7 * 24 * 3600),
                },
                nonce_val,
            )
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

    /// Sign a sub-account rename
    ///
    /// @param subaccount - sub-account pubkey to rename (base58)
    /// @param name - new display name
    /// @param nonce - optional nonce
    #[wasm_bindgen(js_name = signRenameSubAccount)]
    pub fn sign_rename_sub_account(
        &mut self,
        subaccount: &str,
        name: String,
        nonce: Option<f64>,
    ) -> Result<JsValue, JsError> {
        let account =
            Pubkey::from_base58(subaccount).map_err(|e| JsError::new(&e.to_string()))?;
        let nonce_val = nonce.map(|n| n as u64);

        let signed = self
            .inner
            .sign_rename_sub_account(RenameSubAccount { account, name }, nonce_val)
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

fn js_err(message: impl Into<String>) -> JsError {
    JsError::new(&message.into())
}

fn json_obj<'a>(
    value: &'a JsonValue,
    ctx: &str,
) -> Result<&'a serde_json::Map<String, JsonValue>, JsError> {
    value.as_object().ok_or_else(|| js_err(format!("{ctx} must be an object")))
}

fn json_str<'a>(obj: &'a serde_json::Map<String, JsonValue>, key: &str) -> Result<&'a str, JsError> {
    obj.get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| js_err(format!("{key} is required")))
}

fn json_bool(obj: &serde_json::Map<String, JsonValue>, key: &str, default: bool) -> Result<bool, JsError> {
    Ok(obj.get(key).and_then(JsonValue::as_bool).unwrap_or(default))
}

fn json_f64(obj: &serde_json::Map<String, JsonValue>, key: &str) -> Result<f64, JsError> {
    obj.get(key)
        .and_then(JsonValue::as_f64)
        .ok_or_else(|| js_err(format!("{key} is required")))
}

fn json_u32(
    obj: &serde_json::Map<String, JsonValue>,
    key: &str,
    default: Option<u32>,
) -> Result<u32, JsError> {
    match obj.get(key).and_then(JsonValue::as_u64) {
        Some(v) => u32::try_from(v).map_err(|_| js_err(format!("{key} out of range"))),
        None => default.ok_or_else(|| js_err(format!("{key} is required"))),
    }
}

fn json_u64(
    obj: &serde_json::Map<String, JsonValue>,
    key: &str,
    default: Option<u64>,
) -> Result<u64, JsError> {
    match obj.get(key).and_then(JsonValue::as_u64) {
        Some(v) => Ok(v),
        None => default.ok_or_else(|| js_err(format!("{key} is required"))),
    }
}

fn json_pubkey(obj: &serde_json::Map<String, JsonValue>, key: &str) -> Result<Pubkey, JsError> {
    Pubkey::from_base58(json_str(obj, key)?).map_err(|e| js_err(e.to_string()))
}

fn json_hash(obj: &serde_json::Map<String, JsonValue>, key: &str) -> Result<Hash, JsError> {
    Hash::from_base58(json_str(obj, key)?).map_err(|e| js_err(e.to_string()))
}

fn parse_order_input_value(value: JsonValue) -> Result<OrderInput, JsError> {
    serde_json::from_value(value).map_err(|e| js_err(e.to_string()))
}

fn parse_order_item_value(value: JsonValue) -> Result<OrderItem, JsError> {
    let obj = json_obj(&value, "order item")?;
    let (tag, payload) = obj
        .iter()
        .next()
        .ok_or_else(|| js_err("order item cannot be empty"))?;

    if obj.len() != 1 {
        return Err(js_err("order item must have exactly one top-level key"));
    }

    match tag.as_str() {
        "l" => {
            let p = json_obj(payload, "l")?;
            let tif = match json_str(p, "tif")?.to_uppercase().as_str() {
                "GTC" => TimeInForce::Gtc,
                "IOC" => TimeInForce::Ioc,
                "ALO" => TimeInForce::Alo,
                other => return Err(js_err(format!("invalid tif: {other}"))),
            };
            Ok(OrderItem::Order(Order {
                symbol: json_str(p, "c")?.to_string(),
                is_buy: json_bool(p, "b", false)?,
                price: json_f64(p, "px")?,
                size: json_f64(p, "sz")?,
                reduce_only: json_bool(p, "r", false)?,
                iso: json_bool(p, "i", false)?,
                order_type: OrderType::limit(tif),
                client_id: p
                    .get("cloid")
                    .and_then(JsonValue::as_str)
                    .map(Hash::from_base58)
                    .transpose()
                    .map_err(|e| js_err(e.to_string()))?,
            }))
        }
        "m" => {
            let p = json_obj(payload, "m")?;
            Ok(OrderItem::Order(Order {
                symbol: json_str(p, "c")?.to_string(),
                is_buy: json_bool(p, "b", false)?,
                price: 0.0,
                size: json_f64(p, "sz")?,
                reduce_only: json_bool(p, "r", false)?,
                iso: json_bool(p, "i", false)?,
                order_type: OrderType::market(),
                client_id: None,
            }))
        }
        "cx" => {
            let p = json_obj(payload, "cx")?;
            Ok(OrderItem::Cancel(Cancel::new(
                json_str(p, "c")?.to_string(),
                json_hash(p, "oid")?,
            )))
        }
        "mod" => {
            let p = json_obj(payload, "mod")?;
            Ok(OrderItem::Modify(Modify::new(
                json_hash(p, "oid")?,
                json_str(p, "c")?.to_string(),
                json_f64(p, "sz")?,
            )))
        }
        "cxa" => {
            let p = json_obj(payload, "cxa")?;
            let symbols = p
                .get("c")
                .and_then(JsonValue::as_array)
                .ok_or_else(|| js_err("c is required"))?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(ToOwned::to_owned)
                        .ok_or_else(|| js_err("symbol must be a string"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(OrderItem::CancelAll(CancelAll::for_symbols(symbols)))
        }
        "st" => {
            let p = json_obj(payload, "st")?;
            Ok(OrderItem::Stop(Stop {
                symbol: json_str(p, "c")?.to_string(),
                is_buy: json_bool(p, "d", false)?,
                size: json_f64(p, "sz")?,
                trigger_price: json_f64(p, "tr")?,
                limit_price: p.get("lim").and_then(JsonValue::as_f64).unwrap_or(f64::NAN),
                iso: json_bool(p, "i", false)?,
            }))
        }
        "tp" => {
            let p = json_obj(payload, "tp")?;
            Ok(OrderItem::TakeProfit(TakeProfit {
                symbol: json_str(p, "c")?.to_string(),
                is_buy: json_bool(p, "d", false)?,
                size: json_f64(p, "sz")?,
                trigger_price: json_f64(p, "tr")?,
                limit_price: p.get("lim").and_then(JsonValue::as_f64).unwrap_or(f64::NAN),
                iso: json_bool(p, "i", false)?,
            }))
        }
        "rng" => {
            let p = json_obj(payload, "rng")?;
            Ok(OrderItem::RangeOco(RangeOco {
                symbol: json_str(p, "c")?.to_string(),
                is_buy: json_bool(p, "d", false)?,
                size: json_f64(p, "sz")?,
                collar_min: json_f64(p, "pmin")?,
                collar_max: json_f64(p, "pmax")?,
                limit_min: p.get("lmin").and_then(JsonValue::as_f64).unwrap_or(f64::NAN),
                limit_max: p.get("lmax").and_then(JsonValue::as_f64).unwrap_or(f64::NAN),
                iso: json_bool(p, "i", false)?,
            }))
        }
        "trig" => {
            let p = json_obj(payload, "trig")?;
            let nested = p
                .get("actions")
                .and_then(JsonValue::as_array)
                .ok_or_else(|| js_err("actions is required"))?
                .iter()
                .cloned()
                .map(parse_order_item_value)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(OrderItem::TriggerBasket(TriggerBasket {
                symbol: json_str(p, "c")?.to_string(),
                is_buy: json_bool(p, "d", false)?,
                trigger_price: json_f64(p, "tr")?,
                actions: nested,
                iso: json_bool(p, "i", false)?,
            }))
        }
        "of" => {
            let p = json_obj(payload, "of")?;
            let nested = p
                .get("actions")
                .and_then(JsonValue::as_array)
                .ok_or_else(|| js_err("actions is required"))?
                .iter()
                .cloned()
                .map(parse_order_item_value)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(OrderItem::OnFill(OnFill {
                p: json_u32(p, "p", Some(0))?,
                actions: nested,
            }))
        }
        "trl" => {
            let p = json_obj(payload, "trl")?;
            Ok(OrderItem::TrailingStop(TrailingStop {
                symbol: json_str(p, "c")?.to_string(),
                is_buy: json_bool(p, "b", false)?,
                size: json_f64(p, "sz")?,
                trail_bps: json_u32(p, "trb", None)?,
                step_bps: json_u32(p, "stb", None)?,
                limit_price: p.get("lim").and_then(JsonValue::as_f64),
                iso: json_bool(p, "i", false)?,
            }))
        }
        _ => parse_order_input_value(value)?
            .try_into()
            .map_err(js_err),
    }
}

fn parse_action_value(value: JsonValue) -> Result<Action, JsError> {
    let obj = json_obj(&value, "action")?;
    let (tag, payload) = obj
        .iter()
        .next()
        .ok_or_else(|| js_err("action cannot be empty"))?;

    if obj.len() != 1 {
        return Err(js_err("action must have exactly one top-level key"));
    }

    match tag.as_str() {
        "l" | "m" | "cx" | "mod" | "cxa" | "st" | "tp" | "rng" | "trig" | "of" | "trl" => {
            Ok(Action::Order {
                orders: vec![parse_order_item_value(value)?],
            })
        }
        "order" => {
            let orders = match payload {
                JsonValue::Array(items) => items
                    .iter()
                    .cloned()
                    .map(parse_order_item_value)
                    .collect::<Result<Vec<_>, _>>()?,
                _ => vec![parse_order_item_value(payload.clone())?],
            };
            Ok(Action::Order { orders })
        }
        "faucet" => {
            let p = json_obj(payload, "faucet")?;
            let mut faucet = Faucet::new(json_pubkey(p, "u").or_else(|_| json_pubkey(p, "user"))?);
            faucet.amount = p.get("amount").and_then(JsonValue::as_f64);
            Ok(Action::Faucet(faucet))
        }
        "agentWalletCreation" => {
            let p = json_obj(payload, "agentWalletCreation")?;
            Ok(Action::AgentWalletCreation(AgentWallet {
                agent: json_pubkey(p, "a").or_else(|_| json_pubkey(p, "agent"))?,
                delete: json_bool(p, "d", false).or_else(|_| json_bool(p, "delete", false))?,
            }))
        }
        "updateUserSettings" => {
            let p = json_obj(payload, "updateUserSettings")?;
            let leverage_map = p
                .get("m")
                .or_else(|| p.get("maxLeverage"))
                .and_then(JsonValue::as_object)
                .ok_or_else(|| js_err("m is required"))?;
            let max_leverage = leverage_map
                .iter()
                .map(|(k, v)| {
                    v.as_f64()
                        .map(|lev| (k.clone(), lev))
                        .ok_or_else(|| js_err("max leverage values must be numbers"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Action::UpdateUserSettings(UserSettings::new(max_leverage)))
        }
        "px" => {
            let p = json_obj(payload, "px")?;
            Ok(Action::Oracle {
                oracles: vec![OraclePrice {
                    timestamp: json_u64(p, "t", None)?,
                    asset: json_str(p, "c")?.to_string(),
                    price: json_f64(p, "px")?,
                }],
            })
        }
        "o" => {
            let p = json_obj(payload, "o")?;
            let entries = p
                .get("oracles")
                .and_then(JsonValue::as_array)
                .ok_or_else(|| js_err("oracles is required"))?;
            let oracles = entries
                .iter()
                .map(|entry| {
                    let e = json_obj(entry, "oracle")?;
                    Ok(PythOraclePrice {
                        timestamp: json_u64(e, "t", None)?,
                        feed_index: json_u64(e, "fi", None)?,
                        price: json_u64(e, "px", None)?,
                        exponent: e
                            .get("e")
                            .and_then(JsonValue::as_i64)
                            .and_then(|v| i16::try_from(v).ok())
                            .ok_or_else(|| js_err("e is required"))?,
                    })
                })
                .collect::<Result<Vec<_>, JsError>>()?;
            Ok(Action::PythOracle { oracles })
        }
        "whitelistFaucet" => {
            let p = json_obj(payload, "whitelistFaucet")?;
            Ok(Action::WhitelistFaucet(WhitelistFaucet {
                target: json_pubkey(p, "target")?,
                whitelist: json_bool(p, "whitelist", false)?,
            }))
        }
        "createSubAccount" => {
            let p = json_obj(payload, "createSubAccount")?;
            Ok(Action::CreateSubAccount(CreateSubAccount {
                name: json_str(p, "name")?.to_string(),
                margin_symbol: p
                    .get("marginSymbol")
                    .and_then(JsonValue::as_str)
                    .map(ToOwned::to_owned),
                margin_amount: p.get("marginAmount").and_then(JsonValue::as_f64),
            }))
        }
        "removeSubAccount" => {
            let p = json_obj(payload, "removeSubAccount")?;
            Ok(Action::RemoveSubAccount(bulk_keychain::RemoveSubAccount {
                to_remove: json_pubkey(p, "toRemove")?,
            }))
        }
        "renameSubAccount" => {
            let p = json_obj(payload, "renameSubAccount")?;
            Ok(Action::RenameSubAccount(RenameSubAccount {
                account: json_pubkey(p, "account")?,
                name: json_str(p, "name")?.to_string(),
            }))
        }
        "transfer" => {
            let p = json_obj(payload, "transfer")?;
            let kind = match p
                .get("k")
                .and_then(JsonValue::as_str)
                .unwrap_or("internal")
            {
                "internal" => TransferKind::Internal,
                "external" => TransferKind::External,
                other => return Err(js_err(format!("invalid transfer kind: {other}"))),
            };
            Ok(Action::Transfer(Transfer {
                kind,
                from: json_pubkey(p, "from")?,
                to: json_pubkey(p, "to")?,
                margin_symbol: json_str(p, "marginSymbol")?.to_string(),
                margin_amount: json_f64(p, "marginAmount")?,
            }))
        }
        "createMultisig" => {
            let p = json_obj(payload, "createMultisig")?;
            let signers = p
                .get("signers")
                .and_then(JsonValue::as_array)
                .ok_or_else(|| js_err("signers is required"))?
                .iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| js_err("signer must be a string"))
                        .and_then(|s| Pubkey::from_base58(s).map_err(|e| js_err(e.to_string())))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Action::CreateMultisig(CreateMultisig {
                signers,
                threshold: json_u32(p, "threshold", None)?,
                time_lock_secs: json_u32(p, "timeLockSecs", Some(0))?,
                proposal_lifetime_secs: json_u32(p, "proposalLifetimeSecs", Some(7 * 24 * 3600))?,
            }))
        }
        "msp" | "multisigPropose" => {
            let p = json_obj(payload, tag)?;
            let multisig = json_pubkey(p, "m").or_else(|_| json_pubkey(p, "multisig"))?;
            let raw_actions = p
                .get("a")
                .or_else(|| p.get("actions"))
                .and_then(JsonValue::as_array)
                .ok_or_else(|| js_err("actions is required"))?;
            let actions = raw_actions
                .iter()
                .cloned()
                .map(parse_action_value)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Action::MultisigPropose(MultisigPropose::new(multisig, actions)))
        }
        "msa" | "multisigApprove" => {
            let p = json_obj(payload, tag)?;
            Ok(Action::MultisigApprove(MultisigApprove::new(
                json_pubkey(p, "m").or_else(|_| json_pubkey(p, "multisig"))?,
                json_u64(p, "p", p.get("proposalId").and_then(JsonValue::as_u64))?,
            )))
        }
        "msr" | "multisigReject" => {
            let p = json_obj(payload, tag)?;
            Ok(Action::MultisigReject(MultisigReject::new(
                json_pubkey(p, "m").or_else(|_| json_pubkey(p, "multisig"))?,
                json_u64(p, "p", p.get("proposalId").and_then(JsonValue::as_u64))?,
            )))
        }
        "msc" | "multisigCancel" => {
            let p = json_obj(payload, tag)?;
            Ok(Action::MultisigCancel(MultisigCancel::new(
                json_pubkey(p, "m").or_else(|_| json_pubkey(p, "multisig"))?,
                json_u64(p, "p", p.get("proposalId").and_then(JsonValue::as_u64))?,
            )))
        }
        "mse" | "multisigExecute" => {
            let p = json_obj(payload, tag)?;
            Ok(Action::MultisigExecute(MultisigExecute::new(
                json_pubkey(p, "m").or_else(|_| json_pubkey(p, "multisig"))?,
                json_u64(p, "p", p.get("proposalId").and_then(JsonValue::as_u64))?,
            )))
        }
        "msu" | "updateMultisigPolicy" => {
            let p = json_obj(payload, tag)?;
            let signers = p
                .get("signers")
                .and_then(JsonValue::as_array)
                .ok_or_else(|| js_err("signers is required"))?
                .iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| js_err("signer must be a string"))
                        .and_then(|s| Pubkey::from_base58(s).map_err(|e| js_err(e.to_string())))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Action::UpdateMultisigPolicy(UpdateMultisigPolicy {
                multisig: json_pubkey(p, "m").or_else(|_| json_pubkey(p, "multisig"))?,
                signers,
                threshold: json_u32(p, "threshold", None)?,
                time_lock_secs: json_u32(p, "timeLockSecs", Some(0))?,
                proposal_lifetime_secs: json_u32(p, "proposalLifetimeSecs", Some(7 * 24 * 3600))?,
            }))
        }
        _ => Err(js_err(format!("unsupported action type: {tag}"))),
    }
}

fn parse_action_values(value: JsValue) -> Result<Vec<Action>, JsError> {
    let raw: Vec<JsonValue> =
        serde_wasm_bindgen::from_value(value).map_err(|e| js_err(e.to_string()))?;
    raw.into_iter().map(parse_action_value).collect()
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

/// Prepare a sub-account rename for external signing
///
/// @param subaccount - sub-account pubkey to rename (base58)
/// @param name - new display name
/// @param options - { account: string, signer?: string, nonce?: number }
#[wasm_bindgen(js_name = prepareRenameSubAccount)]
pub fn wasm_prepare_rename_sub_account(
    subaccount: &str,
    name: String,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let subaccount =
        Pubkey::from_base58(subaccount).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_rename_sub_account(
        RenameSubAccount {
            account: subaccount,
            name,
        },
        &account,
        signer.as_ref(),
        nonce,
    )
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

/// Prepare a multisig creation for external signing
///
/// @param signers - signer pubkeys (base58)
/// @param threshold - approvals required
/// @param options - { account: string, signer?: string, nonce?: number, timeLockSecs?: number, proposalLifetimeSecs?: number }
#[wasm_bindgen(js_name = prepareCreateMultisig)]
pub fn wasm_prepare_create_multisig(
    signers: JsValue,
    threshold: u32,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CreateMultisigOptions {
        account: String,
        #[serde(default)]
        signer: Option<String>,
        #[serde(default)]
        nonce: Option<f64>,
        #[serde(default)]
        time_lock_secs: Option<u32>,
        #[serde(default)]
        proposal_lifetime_secs: Option<u32>,
    }

    let signer_inputs: Vec<String> =
        serde_wasm_bindgen::from_value(signers).map_err(|e| JsError::new(&e.to_string()))?;
    let signers = signer_inputs
        .into_iter()
        .map(|s| Pubkey::from_base58(&s).map_err(|e| JsError::new(&e.to_string())))
        .collect::<Result<Vec<_>, _>>()?;

    let opts: CreateMultisigOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let create_multisig = CreateMultisig {
        signers,
        threshold,
        time_lock_secs: opts.time_lock_secs.unwrap_or(0),
        proposal_lifetime_secs: opts.proposal_lifetime_secs.unwrap_or(7 * 24 * 3600),
    };

    let prepared = prepare_create_multisig(create_multisig, &account, signer.as_ref(), nonce)
        .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a multisig proposal for external signing
#[wasm_bindgen(js_name = prepareMultisigPropose)]
pub fn wasm_prepare_multisig_propose(
    multisig: &str,
    actions: JsValue,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
    let actions = parse_action_values(actions)?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_multisig_propose(
        MultisigPropose::new(multisig, actions),
        &account,
        signer.as_ref(),
        nonce,
    )
    .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a multisig approval for external signing
#[wasm_bindgen(js_name = prepareMultisigApprove)]
pub fn wasm_prepare_multisig_approve(
    multisig: &str,
    proposal_id: f64,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_multisig_approve(
        MultisigApprove::new(multisig, proposal_id as u64),
        &account,
        signer.as_ref(),
        nonce,
    )
    .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a multisig rejection for external signing
#[wasm_bindgen(js_name = prepareMultisigReject)]
pub fn wasm_prepare_multisig_reject(
    multisig: &str,
    proposal_id: f64,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_multisig_reject(
        MultisigReject::new(multisig, proposal_id as u64),
        &account,
        signer.as_ref(),
        nonce,
    )
    .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a multisig cancellation for external signing
#[wasm_bindgen(js_name = prepareMultisigCancel)]
pub fn wasm_prepare_multisig_cancel(
    multisig: &str,
    proposal_id: f64,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_multisig_cancel(
        MultisigCancel::new(multisig, proposal_id as u64),
        &account,
        signer.as_ref(),
        nonce,
    )
    .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a multisig execution for external signing
#[wasm_bindgen(js_name = prepareMultisigExecute)]
pub fn wasm_prepare_multisig_execute(
    multisig: &str,
    proposal_id: f64,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    let opts: PrepareOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;

    let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let prepared = prepare_multisig_execute(
        MultisigExecute::new(multisig, proposal_id as u64),
        &account,
        signer.as_ref(),
        nonce,
    )
    .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(WasmPreparedMessage { inner: prepared })
}

/// Prepare a multisig policy update for external signing
#[wasm_bindgen(js_name = prepareUpdateMultisigPolicy)]
pub fn wasm_prepare_update_multisig_policy(
    multisig: &str,
    signers: JsValue,
    threshold: u32,
    options: JsValue,
) -> Result<WasmPreparedMessage, JsError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct UpdateMultisigPolicyOptions {
        account: String,
        #[serde(default)]
        signer: Option<String>,
        #[serde(default)]
        nonce: Option<f64>,
        #[serde(default)]
        time_lock_secs: Option<u32>,
        #[serde(default)]
        proposal_lifetime_secs: Option<u32>,
    }

    let multisig = Pubkey::from_base58(multisig).map_err(|e| JsError::new(&e.to_string()))?;
    let signer_inputs: Vec<String> =
        serde_wasm_bindgen::from_value(signers).map_err(|e| JsError::new(&e.to_string()))?;
    let signers = signer_inputs
        .into_iter()
        .map(|s| Pubkey::from_base58(&s).map_err(|e| JsError::new(&e.to_string())))
        .collect::<Result<Vec<_>, _>>()?;

    let opts: UpdateMultisigPolicyOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;
    let account = Pubkey::from_base58(&opts.account).map_err(|e| JsError::new(&e.to_string()))?;
    let signer = opts
        .signer
        .map(|s| Pubkey::from_base58(&s))
        .transpose()
        .map_err(|e| JsError::new(&e.to_string()))?;
    let nonce = opts.nonce.map(|n| n as u64);

    let update = UpdateMultisigPolicy {
        multisig,
        signers,
        threshold,
        time_lock_secs: opts.time_lock_secs.unwrap_or(0),
        proposal_lifetime_secs: opts.proposal_lifetime_secs.unwrap_or(7 * 24 * 3600),
    };

    let prepared = prepare_update_multisig_policy(update, &account, signer.as_ref(), nonce)
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
