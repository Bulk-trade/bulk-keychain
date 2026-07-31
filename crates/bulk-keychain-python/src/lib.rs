//! Python bindings for BULK transaction signing
//!
//! This module provides high-performance Python bindings using PyO3.

use bulk_keychain::{
    compute_order_item_id, prepare_agent_wallet, prepare_all, prepare_approve_commission_fee,
    prepare_create_sub_account, prepare_faucet, prepare_group, prepare_message,
    prepare_remove_sub_account, prepare_revoke_commission_fee, prepare_transfer,
    prepare_update_liquidator_config, Cancel, CancelAll, Commission, CreateSubAccount, Hash,
    Keypair, LiquidatorConfig, LiquidatorInstrumentConfig, Modify, NonceManager, NonceStrategy,
    OnFill, OraclePrice, Order, OrderItem, OrderType, PreparedMessage, Pubkey, PythOraclePrice,
    RangeOco, SignatureDomain, Signer, Stop, TakeProfit, TimeInForce, TrailingStop, Transfer,
    TransferKind, TriggerBasket, UserSettings,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

#[inline]
fn parse_signature_domain(value: &str) -> PyResult<SignatureDomain> {
    value
        .parse()
        .map_err(|error: bulk_keychain::Error| PyValueError::new_err(error.to_string()))
}

// ============================================================================
// Keypair
// ============================================================================

/// Ed25519 keypair for signing transactions
#[pyclass(name = "Keypair")]
pub struct PyKeypair {
    inner: Keypair,
}

#[pymethods]
impl PyKeypair {
    /// Generate a new random keypair
    #[new]
    fn new() -> Self {
        Self {
            inner: Keypair::generate(),
        }
    }

    /// Create from base58-encoded secret key or full keypair
    #[staticmethod]
    fn from_base58(s: &str) -> PyResult<Self> {
        let inner = Keypair::from_base58(s).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Create from raw bytes (32-byte secret or 64-byte keypair)
    #[staticmethod]
    fn from_bytes(bytes: &[u8]) -> PyResult<Self> {
        let inner = Keypair::from_bytes(bytes).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Get the public key as base58 string
    #[getter]
    fn pubkey(&self) -> String {
        self.inner.pubkey().to_base58()
    }

    /// Get the full keypair as base58 (64 bytes)
    fn to_base58(&self) -> String {
        self.inner.to_base58()
    }

    /// Get the full keypair as bytes (64 bytes)
    fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes().to_vec()
    }

    /// Get the secret key as bytes (32 bytes)
    fn secret_key(&self) -> Vec<u8> {
        self.inner.secret_key().to_vec()
    }

    fn __repr__(&self) -> String {
        format!("Keypair(pubkey='{}')", self.pubkey())
    }

    fn __str__(&self) -> String {
        self.pubkey()
    }
}

// ============================================================================
// Signer
// ============================================================================

/// High-performance transaction signer
#[pyclass(name = "Signer")]
pub struct PySigner {
    inner: Signer,
}

#[pymethods]
impl PySigner {
    /// Create a new signer from a keypair
    #[new]
    fn new(keypair: &PyKeypair, signature_domain: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Signer::new(
                keypair.inner.clone(),
                parse_signature_domain(signature_domain)?,
            ),
        })
    }

    /// Create a signer from base58-encoded secret key
    #[staticmethod]
    fn from_base58(s: &str, signature_domain: &str) -> PyResult<Self> {
        let keypair = Keypair::from_base58(s).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: Signer::new(keypair, parse_signature_domain(signature_domain)?),
        })
    }

    /// Create a signer with nonce management
    #[staticmethod]
    fn with_nonce_manager(
        keypair: &PyKeypair,
        strategy: &str,
        signature_domain: &str,
    ) -> PyResult<Self> {
        let nonce_strategy = match strategy {
            "timestamp" => NonceStrategy::Timestamp,
            "counter" => NonceStrategy::Counter,
            "high_frequency" => NonceStrategy::TimestampWithCounter,
            _ => {
                return Err(PyValueError::new_err(
                    "Invalid nonce strategy. Use 'timestamp', 'counter', or 'high_frequency'",
                ))
            }
        };
        let nonce_manager = NonceManager::new(nonce_strategy);
        Ok(Self {
            inner: Signer::with_nonce_manager(
                keypair.inner.clone(),
                parse_signature_domain(signature_domain)?,
                nonce_manager,
            ),
        })
    }

    /// Get the signer's public key
    #[getter]
    fn pubkey(&self) -> String {
        self.inner.pubkey().to_base58()
    }

    /// Enable/disable single-order ID computation.
    fn set_compute_order_id(&mut self, enabled: bool) {
        self.inner.set_order_id(enabled);
    }

    /// Enable/disable batch order ID computation for multi-order transactions.
    fn set_compute_batch_order_ids(&mut self, enabled: bool) {
        self.inner.set_batch_order_ids(enabled);
    }

    /// Whether single-order ID computation is enabled.
    fn computes_order_id(&self) -> bool {
        self.inner.computes_order_id()
    }

    /// Whether batch order ID computation is enabled.
    fn computes_batch_order_ids(&self) -> bool {
        self.inner.computes_batch_order_ids()
    }

    /// Sign raw message bytes and return a base58 Ed25519 signature.
    fn sign_bytes(&self, message: &[u8]) -> String {
        self.inner.sign_bytes(message)
    }

    /// Sign a prepared message and finalize it into a signed transaction.
    ///
    /// This supports agent-wallet flows where prepared["account"] is the trading
    /// account and prepared["signer"] is this signer's pubkey.
    ///
    /// Args:
    ///     prepared: PreparedMessage dict from prepare_* functions
    ///
    /// Returns:
    ///     SignedTransaction dict ready for API submission
    fn sign_prepared(&self, prepared: &Bound<'_, PyDict>) -> PyResult<PyObject> {
        let signer: String = prepared
            .get_item("signer")?
            .ok_or_else(|| PyValueError::new_err("Missing 'signer'"))?
            .extract()?;
        let my_pubkey = self.inner.pubkey().to_base58();
        if signer != my_pubkey {
            return Err(PyValueError::new_err(format!(
                "Prepared message signer {signer} does not match signer pubkey {my_pubkey}"
            )));
        }

        let message_bytes: Vec<u8> = prepared
            .get_item("message_bytes")?
            .ok_or_else(|| PyValueError::new_err("Missing 'message_bytes'"))?
            .extract()?;

        let signature = self.inner.sign_bytes(&message_bytes);
        py_finalize_transaction(prepared, &signature)
    }

    // ========================================================================
    // Simplified API
    // ========================================================================

    /// Sign a single order/cancel/cancelAll
    ///
    /// Most common use case - returns a single signed transaction.
    ///
    /// Example:
    ///     signed = signer.sign({"type": "order", "symbol": "BTC-USD", ...})
    #[pyo3(signature = (order, nonce=None))]
    fn sign(&mut self, order: &Bound<'_, PyAny>, nonce: Option<u64>) -> PyResult<PyObject> {
        let order_item = parse_order_item(order)?;

        let signed = self
            .inner
            .sign(order_item, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign multiple orders - each becomes its own transaction (parallel)
    ///
    /// Optimized for HFT: each order gets independent confirmation/rejection.
    /// Automatically parallelizes when > 10 orders.
    ///
    /// Example:
    ///     signed_txs = signer.sign_all([order1, order2, order3])  # Returns list
    #[pyo3(signature = (orders, base_nonce=None))]
    fn sign_all(&self, orders: &Bound<'_, PyList>, base_nonce: Option<u64>) -> PyResult<PyObject> {
        let order_items: PyResult<Vec<OrderItem>> =
            orders.iter().map(|item| parse_order_item(&item)).collect();
        let order_items = order_items?;

        let signed = self
            .inner
            .sign_all(order_items, base_nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| {
            let list = PyList::empty(py);
            for tx in &signed {
                list.append(signed_to_py(py, tx)?)?;
            }
            Ok(list.into())
        })
    }

    /// Sign multiple orders atomically in ONE transaction
    ///
    /// Use for bracket orders (entry + stop loss + take profit) where
    /// all orders must succeed or fail together.
    ///
    /// Example:
    ///     bracket = [entry, stop_loss, take_profit]
    ///     signed = signer.sign_group(bracket)  # Single transaction
    #[pyo3(signature = (orders, nonce=None))]
    fn sign_group(&mut self, orders: &Bound<'_, PyList>, nonce: Option<u64>) -> PyResult<PyObject> {
        let order_items: PyResult<Vec<OrderItem>> =
            orders.iter().map(|item| parse_order_item(&item)).collect();
        let order_items = order_items?;

        let signed = self
            .inner
            .sign_group(order_items, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    // ========================================================================
    // Other signing methods
    // ========================================================================

    /// Sign a faucet request (testnet only)
    #[pyo3(signature = (nonce=None))]
    fn sign_faucet(&mut self, nonce: Option<u64>) -> PyResult<PyObject> {
        let signed = self
            .inner
            .sign_faucet(nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign agent wallet creation/deletion
    #[pyo3(signature = (agent_pubkey, delete, nonce=None))]
    fn sign_agent_wallet(
        &mut self,
        agent_pubkey: &str,
        delete: bool,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let agent =
            Pubkey::from_base58(agent_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;

        let signed = self
            .inner
            .sign_agent_wallet(agent, delete, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign builder-code recipient approval (`abc`)
    #[pyo3(signature = (to_pubkey, fee, nonce=None))]
    fn sign_approve_commission_fee(
        &mut self,
        to_pubkey: &str,
        fee: u8,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let to =
            Pubkey::from_base58(to_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let signed = self
            .inner
            .sign_approve_commission_fee(to, fee, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign builder-code recipient approval (`abc`)
    #[pyo3(signature = (to_pubkey, fee, nonce=None))]
    fn sign_approve_builder_code(
        &mut self,
        to_pubkey: &str,
        fee: u8,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        self.sign_approve_commission_fee(to_pubkey, fee, nonce)
    }

    /// Sign builder-code recipient revocation (`rbc`)
    #[pyo3(signature = (to_pubkey, nonce=None))]
    fn sign_revoke_commission_fee(
        &mut self,
        to_pubkey: &str,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let to =
            Pubkey::from_base58(to_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let signed = self
            .inner
            .sign_revoke_commission_fee(to, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign builder-code recipient revocation (`rbc`)
    #[pyo3(signature = (to_pubkey, nonce=None))]
    fn sign_revoke_builder_code(
        &mut self,
        to_pubkey: &str,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        self.sign_revoke_commission_fee(to_pubkey, nonce)
    }

    /// Sign user settings update
    #[pyo3(signature = (max_leverage, nonce=None))]
    fn sign_user_settings(
        &mut self,
        max_leverage: Vec<(String, f64)>,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let settings = UserSettings::new(max_leverage);

        let signed = self
            .inner
            .sign_user_settings(settings, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign a liquidator config update (`liq`)
    #[pyo3(signature = (config, nonce=None))]
    fn sign_update_liquidator_config(
        &mut self,
        config: &Bound<'_, PyAny>,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let config = parse_liquidator_config(config)?;

        let signed = self
            .inner
            .sign_update_liquidator_config(config, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign one or more oracle price updates (`px`)
    #[pyo3(signature = (oracles, nonce=None))]
    fn sign_oracle_prices(
        &mut self,
        oracles: Vec<(u64, String, f64)>,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let oracle_prices: Vec<OraclePrice> = oracles
            .into_iter()
            .map(|(timestamp, asset, price)| OraclePrice {
                timestamp,
                asset,
                price,
            })
            .collect();

        let signed = self
            .inner
            .sign_oracle_prices(oracle_prices, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign a batch Pyth oracle update (`o`)
    #[pyo3(signature = (oracles, nonce=None))]
    fn sign_pyth_oracle(
        &mut self,
        oracles: Vec<(u64, u64, u64, i16)>,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let pyth_oracles: Vec<PythOraclePrice> = oracles
            .into_iter()
            .map(|(timestamp, feed_index, price, exponent)| PythOraclePrice {
                timestamp,
                feed_index,
                price,
                exponent,
            })
            .collect();

        let signed = self
            .inner
            .sign_pyth_oracle(pyth_oracles, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign a sub-account removal
    #[pyo3(signature = (to_remove, nonce=None))]
    fn sign_remove_sub_account(
        &mut self,
        to_remove: &str,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let target =
            Pubkey::from_base58(to_remove).map_err(|e| PyValueError::new_err(e.to_string()))?;

        let signed = self
            .inner
            .sign_remove_sub_account(target, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign a margin transfer between accounts.
    ///
    /// `kind` is "internal" (default) or "external".
    #[pyo3(signature = (from_pubkey, to_pubkey, margin_amount, kind=None, nonce=None))]
    fn sign_transfer(
        &mut self,
        from_pubkey: &str,
        to_pubkey: &str,
        margin_amount: f64,
        kind: Option<&str>,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let from =
            Pubkey::from_base58(from_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let to =
            Pubkey::from_base58(to_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let kind = parse_transfer_kind(kind)?;

        let transfer = Transfer {
            kind,
            from,
            to,
            margin_amount,
        };

        let signed = self
            .inner
            .sign_transfer(transfer, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign a sub-account creation (optional initial margin transfer)
    #[pyo3(signature = (name, margin_amount=None, nonce=None))]
    fn sign_create_sub_account(
        &mut self,
        name: String,
        margin_amount: Option<f64>,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let sub_account = CreateSubAccount {
            name,
            margin_amount,
        };

        let signed = self
            .inner
            .sign_create_sub_account(sub_account, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    /// Sign whitelist/un-whitelist faucet access (`whitelistFaucet`)
    #[pyo3(signature = (target_pubkey, whitelist, nonce=None))]
    fn sign_whitelist_faucet(
        &mut self,
        target_pubkey: &str,
        whitelist: bool,
        nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        let target =
            Pubkey::from_base58(target_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;

        let signed = self
            .inner
            .sign_whitelist_faucet(target, whitelist, nonce)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Python::with_gil(|py| signed_to_py(py, &signed))
    }

    // ========================================================================
    // Legacy methods (deprecated, kept for backward compatibility)
    // ========================================================================

    /// Deprecated: Use sign() for single, sign_all() for batch, sign_group() for atomic
    #[pyo3(signature = (orders, nonce=None))]
    fn sign_order(&mut self, orders: &Bound<'_, PyList>, nonce: Option<u64>) -> PyResult<PyObject> {
        self.sign_group(orders, nonce)
    }

    /// Deprecated: Use sign_all() instead
    #[pyo3(signature = (batches, base_nonce=None))]
    fn sign_orders_batch(
        &self,
        batches: &Bound<'_, PyList>,
        base_nonce: Option<u64>,
    ) -> PyResult<PyObject> {
        #[allow(deprecated)]
        {
            let order_batches: PyResult<Vec<Vec<OrderItem>>> = batches
                .iter()
                .map(|batch| {
                    let batch_list = batch.downcast::<PyList>()?;
                    batch_list
                        .iter()
                        .map(|item| parse_order_item(&item))
                        .collect()
                })
                .collect();
            let order_batches = order_batches?;

            let signed = self
                .inner
                .sign_orders_batch(order_batches, base_nonce)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;

            Python::with_gil(|py| {
                let list = PyList::empty(py);
                for tx in &signed {
                    list.append(signed_to_py(py, tx)?)?;
                }
                Ok(list.into())
            })
        }
    }

    fn __repr__(&self) -> String {
        format!("Signer(pubkey='{}')", self.pubkey())
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn parse_liquidator_config(obj: &Bound<'_, PyAny>) -> PyResult<LiquidatorConfig> {
    let dict = obj.downcast::<PyDict>()?;

    let f64_field = |d: &Bound<'_, PyDict>, key: &str| -> PyResult<f64> {
        Ok(d.get_item(key)?
            .map(|v| v.extract::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0))
    };

    let instruments = match dict.get_item("instruments")? {
        Some(list) => list
            .downcast::<PyList>()?
            .iter()
            .map(|entry| {
                let inst = entry.downcast::<PyDict>()?;
                let rampup = f64_field(inst, "volume_rampup")?;
                Ok(LiquidatorInstrumentConfig {
                    symbol: inst
                        .get_item("symbol")?
                        .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                        .extract()?,
                    max_exposure: f64_field(inst, "max_exposure")?,
                    premium_min: f64_field(inst, "premium_min")?,
                    fee: f64_field(inst, "fee")?,
                    volume_percent: f64_field(inst, "volume_percent")?,
                    volume_min: f64_field(inst, "volume_min")?,
                    volume_rampup: if rampup > 0.0 { rampup as u64 } else { 0 },
                    max_adl_notional: f64_field(inst, "max_adl_notional")?,
                    max_adl_percent: f64_field(inst, "max_adl_percent")?,
                })
            })
            .collect::<PyResult<Vec<_>>>()?,
        None => Vec::new(),
    };

    Ok(LiquidatorConfig {
        cross_exposure: f64_field(dict, "cross_exposure")?,
        scoring_skew: f64_field(dict, "scoring_skew")?,
        toxicity: f64_field(dict, "toxicity")?,
        instruments,
    })
}

fn parse_order_item(obj: &Bound<'_, PyAny>) -> PyResult<OrderItem> {
    let dict = obj.downcast::<PyDict>()?;

    let item_type: String = dict
        .get_item("type")?
        .ok_or_else(|| PyValueError::new_err("Missing 'type' field"))?
        .extract()?;

    match item_type.as_str() {
        "order" => {
            let symbol: String = dict
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                .extract()?;
            let is_buy: bool = dict
                .get_item("is_buy")?
                .ok_or_else(|| PyValueError::new_err("Missing 'is_buy'"))?
                .extract()?;
            let price: f64 = dict
                .get_item("price")?
                .ok_or_else(|| PyValueError::new_err("Missing 'price'"))?
                .extract()?;
            let size: f64 = dict
                .get_item("size")?
                .ok_or_else(|| PyValueError::new_err("Missing 'size'"))?
                .extract()?;
            let reduce_only: bool = dict
                .get_item("reduce_only")?
                .map(|v| v.extract().unwrap_or(false))
                .unwrap_or(false);
            let iso: bool = dict
                .get_item("iso")?
                .map(|v| v.extract().unwrap_or(false))
                .unwrap_or(false);

            let order_type = if let Some(ot) = dict.get_item("order_type")? {
                let ot_dict = ot.downcast::<PyDict>()?;
                let ot_type: String = ot_dict
                    .get_item("type")?
                    .ok_or_else(|| PyValueError::new_err("Missing order_type.type"))?
                    .extract()?;

                match ot_type.as_str() {
                    "limit" => {
                        let tif_str: String = ot_dict
                            .get_item("tif")?
                            .map(|v| v.extract().unwrap_or("GTC".to_string()))
                            .unwrap_or_else(|| "GTC".to_string());
                        let tif = match tif_str.to_uppercase().as_str() {
                            "GTC" => TimeInForce::Gtc,
                            "IOC" => TimeInForce::Ioc,
                            "ALO" => TimeInForce::Alo,
                            _ => {
                                return Err(PyValueError::new_err(format!(
                                    "Invalid tif: {}",
                                    tif_str
                                )))
                            }
                        };
                        OrderType::limit(tif)
                    }
                    "trigger" | "market" => {
                        let is_market: bool = ot_dict
                            .get_item("is_market")?
                            .map(|v| v.extract().unwrap_or(true))
                            .unwrap_or(true);
                        let trigger_px: f64 = ot_dict
                            .get_item("trigger_px")?
                            .map(|v| v.extract().unwrap_or(0.0))
                            .unwrap_or(0.0);
                        OrderType::Trigger {
                            is_market,
                            trigger_px,
                        }
                    }
                    _ => {
                        return Err(PyValueError::new_err(format!(
                            "Invalid order_type: {}",
                            ot_type
                        )))
                    }
                }
            } else {
                OrderType::limit(TimeInForce::Gtc)
            };

            let client_id = if let Some(cid) = dict.get_item("client_id")? {
                let cid_str: String = cid.extract()?;
                Some(
                    Hash::from_base58(&cid_str)
                        .map_err(|e| PyValueError::new_err(e.to_string()))?,
                )
            } else {
                None
            };

            Ok(OrderItem::Order(Order {
                symbol,
                is_buy,
                price,
                size,
                reduce_only,
                iso,
                order_type,
                client_id,
                commission: {
                    if dict.get_item("commission")?.is_some() {
                        return Err(PyValueError::new_err(
                            "commission was renamed to builder_code",
                        ));
                    }
                    parse_commission(dict.get_item("builder_code")?)?
                },
            }))
        }
        "cancel" => {
            let symbol: String = dict
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                .extract()?;
            let order_id_str: String = dict
                .get_item("order_id")?
                .ok_or_else(|| PyValueError::new_err("Missing 'order_id'"))?
                .extract()?;
            let order_id = Hash::from_base58(&order_id_str)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;

            Ok(OrderItem::Cancel(Cancel::new(symbol, order_id)))
        }
        "modify" => {
            let symbol: String = dict
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                .extract()?;
            let order_id_str: String = dict
                .get_item("order_id")?
                .ok_or_else(|| PyValueError::new_err("Missing 'order_id'"))?
                .extract()?;
            let amount: f64 = dict
                .get_item("amount")?
                .ok_or_else(|| PyValueError::new_err("Missing 'amount'"))?
                .extract()?;
            let order_id = Hash::from_base58(&order_id_str)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            Ok(OrderItem::Modify(Modify::new(order_id, symbol, amount)))
        }
        "cancel_all" => {
            let symbols: Vec<String> = dict
                .get_item("symbols")?
                .map(|v| v.extract().unwrap_or_default())
                .unwrap_or_default();

            Ok(OrderItem::CancelAll(CancelAll::for_symbols(symbols)))
        }
        "stop" | "st" => {
            let symbol: String = dict
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                .extract()?;
            let is_buy: bool = dict
                .get_item("is_buy")?
                .ok_or_else(|| PyValueError::new_err("Missing 'is_buy'"))?
                .extract()?;
            let size: f64 = dict
                .get_item("size")?
                .ok_or_else(|| PyValueError::new_err("Missing 'size'"))?
                .extract()?;
            let trigger_price: f64 = dict
                .get_item("trigger_price")?
                .ok_or_else(|| PyValueError::new_err("Missing 'trigger_price'"))?
                .extract()?;
            let limit_price: f64 = dict
                .get_item("limit_price")?
                .map(|v| v.extract().unwrap_or(f64::NAN))
                .unwrap_or(f64::NAN);
            let iso: bool = dict
                .get_item("iso")?
                .map(|v| v.extract().unwrap_or(false))
                .unwrap_or(false);
            Ok(OrderItem::Stop(Stop {
                symbol,
                is_buy,
                size,
                trigger_price,
                limit_price,
                iso,
            }))
        }
        "take_profit" | "tp" => {
            let symbol: String = dict
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                .extract()?;
            let is_buy: bool = dict
                .get_item("is_buy")?
                .ok_or_else(|| PyValueError::new_err("Missing 'is_buy'"))?
                .extract()?;
            let size: f64 = dict
                .get_item("size")?
                .ok_or_else(|| PyValueError::new_err("Missing 'size'"))?
                .extract()?;
            let trigger_price: f64 = dict
                .get_item("trigger_price")?
                .ok_or_else(|| PyValueError::new_err("Missing 'trigger_price'"))?
                .extract()?;
            let limit_price: f64 = dict
                .get_item("limit_price")?
                .map(|v| v.extract().unwrap_or(f64::NAN))
                .unwrap_or(f64::NAN);
            let iso: bool = dict
                .get_item("iso")?
                .map(|v| v.extract().unwrap_or(false))
                .unwrap_or(false);
            Ok(OrderItem::TakeProfit(TakeProfit {
                symbol,
                is_buy,
                size,
                trigger_price,
                limit_price,
                iso,
            }))
        }
        "range" | "rng" => {
            let symbol: String = dict
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                .extract()?;
            let is_buy: bool = dict
                .get_item("is_buy")?
                .ok_or_else(|| PyValueError::new_err("Missing 'is_buy'"))?
                .extract()?;
            let size: f64 = dict
                .get_item("size")?
                .ok_or_else(|| PyValueError::new_err("Missing 'size'"))?
                .extract()?;
            let collar_min: f64 = dict
                .get_item("pmin")?
                .ok_or_else(|| PyValueError::new_err("Missing 'pmin'"))?
                .extract()?;
            let collar_max: f64 = dict
                .get_item("pmax")?
                .ok_or_else(|| PyValueError::new_err("Missing 'pmax'"))?
                .extract()?;
            let limit_min: f64 = dict
                .get_item("lmin")?
                .map(|v| v.extract().unwrap_or(f64::NAN))
                .unwrap_or(f64::NAN);
            let limit_max: f64 = dict
                .get_item("lmax")?
                .map(|v| v.extract().unwrap_or(f64::NAN))
                .unwrap_or(f64::NAN);
            let iso: bool = dict
                .get_item("iso")?
                .map(|v| v.extract().unwrap_or(false))
                .unwrap_or(false);
            Ok(OrderItem::RangeOco(RangeOco {
                symbol,
                is_buy,
                size,
                collar_min,
                collar_max,
                limit_min,
                limit_max,
                iso,
            }))
        }
        "trig" => {
            if dict.get_item("iso")?.is_some() {
                return Err(PyValueError::new_err(
                    "trig.iso is not supported; remove the top-level iso field",
                ));
            }
            let symbol: String = dict
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                .extract()?;
            let is_buy: bool = dict
                .get_item("is_buy")?
                .ok_or_else(|| PyValueError::new_err("Missing 'is_buy'"))?
                .extract()?;
            let trigger_price: f64 = dict
                .get_item("trigger_price")?
                .ok_or_else(|| PyValueError::new_err("Missing 'trigger_price'"))?
                .extract()?;
            let raw_actions = dict
                .get_item("actions")?
                .ok_or_else(|| PyValueError::new_err("Missing 'actions'"))?;
            let actions_list = raw_actions.downcast::<PyList>()?;
            let actions: PyResult<Vec<OrderItem>> =
                actions_list.iter().map(|a| parse_order_item(&a)).collect();
            Ok(OrderItem::TriggerBasket(TriggerBasket {
                symbol,
                is_buy,
                trigger_price,
                actions: actions?,
            }))
        }
        "on_fill" | "of" => {
            let trigger = dict
                .get_item("trigger")?
                .ok_or_else(|| PyValueError::new_err("Missing 'trigger'"))
                .and_then(|v| parse_order_item(&v))?;
            let raw_actions = dict
                .get_item("actions")?
                .ok_or_else(|| PyValueError::new_err("Missing 'actions'"))?;
            let actions_list = raw_actions.downcast::<PyList>()?;
            let actions: PyResult<Vec<OrderItem>> =
                actions_list.iter().map(|a| parse_order_item(&a)).collect();
            Ok(OrderItem::OnFill(OnFill {
                trigger: Box::new(trigger),
                actions: actions?,
            }))
        }
        "trailing_stop" | "trl" => {
            let symbol: String = dict
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'symbol'"))?
                .extract()?;
            let is_buy: bool = dict
                .get_item("is_buy")?
                .ok_or_else(|| PyValueError::new_err("Missing 'is_buy'"))?
                .extract()?;
            let size: f64 = dict
                .get_item("size")?
                .ok_or_else(|| PyValueError::new_err("Missing 'size'"))?
                .extract()?;
            let trail_bps: u32 = dict
                .get_item("trail_bps")?
                .ok_or_else(|| PyValueError::new_err("Missing 'trail_bps'"))?
                .extract()?;
            let step_bps: u32 = dict
                .get_item("step_bps")?
                .ok_or_else(|| PyValueError::new_err("Missing 'step_bps'"))?
                .extract()?;
            let limit_price: Option<f64> =
                dict.get_item("limit_price")?.and_then(|v| v.extract().ok());
            let iso: bool = dict
                .get_item("iso")?
                .map(|v| v.extract().unwrap_or(false))
                .unwrap_or(false);
            Ok(OrderItem::TrailingStop(TrailingStop {
                symbol,
                is_buy,
                size,
                trail_bps,
                step_bps,
                limit_price,
                iso,
            }))
        }
        _ => Err(PyValueError::new_err(format!(
            "Invalid item type: {}",
            item_type
        ))),
    }
}

fn parse_order_item_for_id(obj: &Bound<'_, PyAny>) -> PyResult<OrderItem> {
    let dict = obj.downcast::<PyDict>()?;
    if dict.get_item("type")?.is_some() {
        return parse_order_item(obj);
    }
    parse_compact_order_item(dict)
}

fn parse_commission(value: Option<Bound<'_, PyAny>>) -> PyResult<Option<Commission>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_none() {
        return Err(PyValueError::new_err(
            "builder_code must be omitted or an object",
        ));
    }
    let dict = value.downcast::<PyDict>()?;
    let to: String = dict
        .get_item("to")?
        .ok_or_else(|| PyValueError::new_err("Missing builder_code.to"))?
        .extract()?;
    let fee: u8 = dict
        .get_item("fee")?
        .ok_or_else(|| PyValueError::new_err("Missing builder_code.fee"))?
        .extract()?;
    Commission::new(
        Pubkey::from_base58(&to).map_err(|e| PyValueError::new_err(e.to_string()))?,
        fee,
    )
    .map(Some)
    .map_err(|e| PyValueError::new_err(e.to_string()))
}

fn parse_compact_order_item(dict: &Bound<'_, PyDict>) -> PyResult<OrderItem> {
    if let Some(limit_obj) = dict.get_item("l")? {
        let limit = limit_obj.downcast::<PyDict>()?;
        let symbol: String = limit
            .get_item("c")?
            .ok_or_else(|| PyValueError::new_err("Missing 'l.c'"))?
            .extract()?;
        let is_buy: bool = limit
            .get_item("b")?
            .ok_or_else(|| PyValueError::new_err("Missing 'l.b'"))?
            .extract()?;
        let price: f64 = limit
            .get_item("px")?
            .ok_or_else(|| PyValueError::new_err("Missing 'l.px'"))?
            .extract()?;
        let size: f64 = limit
            .get_item("sz")?
            .ok_or_else(|| PyValueError::new_err("Missing 'l.sz'"))?
            .extract()?;
        let reduce_only: bool = limit
            .get_item("r")?
            .map(|v| v.extract().unwrap_or(false))
            .unwrap_or(false);
        let iso: bool = limit
            .get_item("i")?
            .map(|v| v.extract().unwrap_or(false))
            .unwrap_or(false);
        let tif_str: String = limit
            .get_item("tif")?
            .map(|v| v.extract().unwrap_or("GTC".to_string()))
            .unwrap_or_else(|| "GTC".to_string());
        let tif = match tif_str.to_uppercase().as_str() {
            "GTC" => TimeInForce::Gtc,
            "IOC" => TimeInForce::Ioc,
            "ALO" => TimeInForce::Alo,
            _ => return Err(PyValueError::new_err(format!("Invalid l.tif: {}", tif_str))),
        };
        let client_id = if let Some(cloid) = limit.get_item("cloid")? {
            let cloid_str: String = cloid.extract()?;
            Some(Hash::from_base58(&cloid_str).map_err(|e| PyValueError::new_err(e.to_string()))?)
        } else {
            None
        };

        return Ok(OrderItem::Order(Order {
            symbol,
            is_buy,
            price,
            size,
            reduce_only,
            iso,
            order_type: OrderType::Limit { tif },
            client_id,
            commission: {
                if limit.get_item("commission")?.is_some() {
                    return Err(PyValueError::new_err(
                        "commission was renamed to builder_code",
                    ));
                }
                parse_commission(limit.get_item("builder_code")?)?
            },
        }));
    }

    if let Some(market_obj) = dict.get_item("m")? {
        let market = market_obj.downcast::<PyDict>()?;
        let symbol: String = market
            .get_item("c")?
            .ok_or_else(|| PyValueError::new_err("Missing 'm.c'"))?
            .extract()?;
        let is_buy: bool = market
            .get_item("b")?
            .ok_or_else(|| PyValueError::new_err("Missing 'm.b'"))?
            .extract()?;
        let size: f64 = market
            .get_item("sz")?
            .ok_or_else(|| PyValueError::new_err("Missing 'm.sz'"))?
            .extract()?;
        let reduce_only: bool = market
            .get_item("r")?
            .map(|v| v.extract().unwrap_or(false))
            .unwrap_or(false);
        let iso: bool = market
            .get_item("i")?
            .map(|v| v.extract().unwrap_or(false))
            .unwrap_or(false);

        return Ok(OrderItem::Order(Order {
            symbol,
            is_buy,
            price: 0.0,
            size,
            reduce_only,
            iso,
            order_type: OrderType::Trigger {
                is_market: true,
                trigger_px: 0.0,
            },
            client_id: None,
            commission: {
                if market.get_item("commission")?.is_some() {
                    return Err(PyValueError::new_err(
                        "commission was renamed to builder_code",
                    ));
                }
                parse_commission(market.get_item("builder_code")?)?
            },
        }));
    }

    if let Some(cancel_obj) = dict.get_item("cx")? {
        let cancel = cancel_obj.downcast::<PyDict>()?;
        let symbol: String = cancel
            .get_item("c")?
            .ok_or_else(|| PyValueError::new_err("Missing 'cx.c'"))?
            .extract()?;
        let order_id_str: String = cancel
            .get_item("oid")?
            .ok_or_else(|| PyValueError::new_err("Missing 'cx.oid'"))?
            .extract()?;
        let order_id =
            Hash::from_base58(&order_id_str).map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(OrderItem::Cancel(Cancel::new(symbol, order_id)));
    }

    if let Some(mod_obj) = dict.get_item("mod")? {
        let modify = mod_obj.downcast::<PyDict>()?;
        let symbol: String = if let Some(v) = modify.get_item("c")? {
            v.extract()?
        } else {
            modify
                .get_item("symbol")?
                .ok_or_else(|| PyValueError::new_err("Missing 'mod.c'"))?
                .extract()?
        };
        let order_id_str: String = modify
            .get_item("oid")?
            .ok_or_else(|| PyValueError::new_err("Missing 'mod.oid'"))?
            .extract()?;
        let amount: f64 = if let Some(v) = modify.get_item("sz")? {
            v.extract()?
        } else {
            modify
                .get_item("amount")?
                .ok_or_else(|| PyValueError::new_err("Missing 'mod.sz'"))?
                .extract()?
        };
        let order_id =
            Hash::from_base58(&order_id_str).map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(OrderItem::Modify(Modify::new(order_id, symbol, amount)));
    }

    if let Some(cancel_all_obj) = dict.get_item("cxa")? {
        let cancel_all = cancel_all_obj.downcast::<PyDict>()?;
        let symbols: Vec<String> = cancel_all
            .get_item("c")?
            .map(|v| v.extract().unwrap_or_default())
            .unwrap_or_default();
        return Ok(OrderItem::CancelAll(CancelAll::for_symbols(symbols)));
    }

    Err(PyValueError::new_err(
        "Invalid order JSON. Expected simplified {'type': ...} or compact {'l'|'m'|'cx'|'mod'|'cxa'}",
    ))
}

fn signed_to_py(py: Python<'_>, signed: &bulk_keychain::SignedTransaction) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    dict.set_item(
        "actions",
        json_to_py(py, &serde_json::Value::Array(signed.actions.clone()))?,
    )?;
    dict.set_item("nonce", signed.nonce)?;
    dict.set_item("account", &signed.account)?;
    dict.set_item("signer", &signed.signer)?;
    dict.set_item("signature", &signed.signature)?;
    // Include order_id if available (single order transactions)
    if let Some(ref order_id) = signed.order_id {
        dict.set_item("order_id", order_id)?;
    }
    // Include order_ids if available (multi-order transactions)
    if let Some(ref order_ids) = signed.order_ids {
        dict.set_item("order_ids", order_ids)?;
    }
    Ok(dict.into())
}

fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok((*b).into_pyobject(py)?.to_owned().unbind().into()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.to_owned().unbind().into())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.to_owned().unbind().into())
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.to_owned().unbind().into()),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

// ============================================================================
// Module functions
// ============================================================================

/// Generate a random hash (for client order IDs)
#[pyfunction]
fn random_hash() -> String {
    Hash::random().to_base58()
}

/// Get current timestamp in milliseconds
#[pyfunction]
fn current_timestamp() -> u64 {
    bulk_keychain::nonce::current_timestamp_millis()
}

/// Validate a base58-encoded public key
#[pyfunction]
fn validate_pubkey(s: &str) -> bool {
    Pubkey::from_base58(s).is_ok()
}

/// Validate a base58-encoded hash
#[pyfunction]
fn validate_hash(s: &str) -> bool {
    Hash::from_base58(s).is_ok()
}

/// Compute SHA256 hash from raw bytes.
///
/// This is a raw utility and does not apply BULK order-ID canonicalization.
#[pyfunction]
fn compute_order_id(wincode_bytes: &[u8]) -> String {
    Hash::from_wincode_bytes(wincode_bytes).to_base58()
}

/// Compute order ID from an order JSON object without a private key.
///
/// Supports:
/// - Simplified shape: {"type": "order", ...}
/// - Compact API shape: {"l": {...}} / {"m": {...}}
///
/// Returns `None` for non-order actions (cancel/modify/cancel-all).
#[pyfunction]
#[pyo3(signature = (order, nonce, account))]
fn compute_order_id_from_order(
    order: &Bound<'_, PyAny>,
    nonce: u64,
    account: &str,
) -> PyResult<Option<String>> {
    let item = parse_order_item_for_id(order)?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(compute_order_item_id(&item, nonce, &account_pk).map(|id| id.to_base58()))
}

// ============================================================================
// External Wallet Support - Prepare/Finalize API
// ============================================================================

fn prepared_to_py(py: Python<'_>, prepared: &PreparedMessage) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    // Raw bytes as Python bytes
    dict.set_item(
        "message_bytes",
        pyo3::types::PyBytes::new(py, &prepared.message_bytes),
    )?;
    // Format helpers
    dict.set_item("message_base58", prepared.message_base58())?;
    dict.set_item("message_base64", prepared.message_base64())?;
    dict.set_item("message_hex", prepared.message_hex())?;
    // Metadata
    if let Some(ref order_id) = prepared.order_id {
        dict.set_item("order_id", order_id)?;
    }
    if let Some(ref order_ids) = prepared.order_ids {
        dict.set_item("order_ids", order_ids)?;
    }
    dict.set_item(
        "actions",
        json_to_py(py, &serde_json::Value::Array(prepared.actions.clone()))?,
    )?;
    dict.set_item("account", &prepared.account)?;
    dict.set_item("signer", &prepared.signer)?;
    dict.set_item("nonce", prepared.nonce)?;
    Ok(dict.into())
}

/// Prepare a single order for external wallet signing
///
/// Use this when you don't have access to the private key and need
/// to sign with an external wallet.
///
/// Args:
///     order: Order dict with type, symbol, is_buy, price, size, etc.
///     account: Account public key (base58)
///     signer: Signer public key (defaults to account)
///     nonce: Transaction nonce (defaults to current timestamp)
///
/// Returns:
///     PreparedMessage dict with message_bytes to sign
///
/// Example:
///     prepared = prepare_order(order, "account_pubkey")
///     signature = wallet.sign_message(prepared["message_bytes"])
///     signed = finalize_transaction(prepared, signature)
#[pyfunction]
#[pyo3(signature = (order, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_order(
    order: &Bound<'_, PyAny>,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    // If on_fill is present, wrap the parent inline as the OnFill trigger.
    let dict = order.downcast::<PyDict>().ok();
    let on_fill_obj = dict
        .as_ref()
        .and_then(|d| d.get_item("on_fill").ok().flatten());

    let prepared = if let Some(of_obj) = on_fill_obj {
        let parent = parse_order_item(order)?;
        let of_dict = of_obj.downcast::<PyDict>()?;
        let raw_actions = of_dict
            .get_item("actions")?
            .ok_or_else(|| PyValueError::new_err("Missing on_fill.actions"))?;
        let actions_list = raw_actions.downcast::<PyList>()?;
        let consequents: PyResult<Vec<OrderItem>> =
            actions_list.iter().map(|a| parse_order_item(&a)).collect();
        let of_item = OrderItem::OnFill(OnFill {
            trigger: Box::new(parent),
            actions: consequents?,
        });
        prepare_message(
            of_item,
            signature_domain,
            &account_pk,
            signer_pk.as_ref(),
            nonce,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?
    } else {
        let order_item = parse_order_item(order)?;
        prepare_message(
            order_item,
            signature_domain,
            &account_pk,
            signer_pk.as_ref(),
            nonce,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?
    };

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare multiple orders - each becomes its own transaction (parallel)
///
/// Optimized for HFT: each order gets independent confirmation/rejection.
///
/// Example:
///     prepared_list = prepare_all_orders([order1, order2], "account_pubkey")
#[pyfunction]
#[pyo3(signature = (orders, signature_domain, account, signer=None, base_nonce=None))]
fn py_prepare_all_orders(
    orders: &Bound<'_, PyList>,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    base_nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let order_items: PyResult<Vec<OrderItem>> =
        orders.iter().map(|item| parse_order_item(&item)).collect();
    let order_items = order_items?;

    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_all(
        order_items,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        base_nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| {
        let list = PyList::empty(py);
        for p in &prepared {
            list.append(prepared_to_py(py, p)?)?;
        }
        Ok(list.into())
    })
}

/// Prepare multiple orders as ONE atomic transaction
///
/// Use for bracket orders (entry + stop loss + take profit).
///
/// Example:
///     bracket = [entry, stop_loss, take_profit]
///     prepared = prepare_order_group(bracket, "account_pubkey")
#[pyfunction]
#[pyo3(signature = (orders, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_order_group(
    orders: &Bound<'_, PyList>,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let order_items: PyResult<Vec<OrderItem>> =
        orders.iter().map(|item| parse_order_item(&item)).collect();
    let order_items = order_items?;

    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_group(
        order_items,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare agent wallet creation for external signing
///
/// Example:
///     prepared = prepare_agent_wallet_auth(agent_pubkey, False, "account_pubkey")
#[pyfunction]
#[pyo3(signature = (agent_pubkey, delete, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_agent_wallet_auth(
    agent_pubkey: &str,
    delete: bool,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let agent =
        Pubkey::from_base58(agent_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_agent_wallet(
        &agent,
        delete,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare a liquidator config update for external signing
#[pyfunction]
#[pyo3(signature = (config, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_update_liquidator_config(
    config: &Bound<'_, PyAny>,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let config = parse_liquidator_config(config)?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_update_liquidator_config(
        config,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare builder-code recipient approval for external signing
#[pyfunction]
#[pyo3(signature = (to_pubkey, fee, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_approve_commission_fee(
    to_pubkey: &str,
    fee: u8,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let to = Pubkey::from_base58(to_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_approve_commission_fee(
        &to,
        fee,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare builder-code recipient approval for external signing
#[pyfunction]
#[pyo3(signature = (to_pubkey, fee, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_approve_builder_code(
    to_pubkey: &str,
    fee: u8,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    py_prepare_approve_commission_fee(to_pubkey, fee, signature_domain, account, signer, nonce)
}

/// Prepare builder-code recipient revocation for external signing
#[pyfunction]
#[pyo3(signature = (to_pubkey, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_revoke_commission_fee(
    to_pubkey: &str,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let to = Pubkey::from_base58(to_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_revoke_commission_fee(
        &to,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare builder-code recipient revocation for external signing
#[pyfunction]
#[pyo3(signature = (to_pubkey, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_revoke_builder_code(
    to_pubkey: &str,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    py_prepare_revoke_commission_fee(to_pubkey, signature_domain, account, signer, nonce)
}

/// Prepare faucet request for external signing
#[pyfunction]
#[pyo3(signature = (signature_domain, account, signer=None, nonce=None))]
fn py_prepare_faucet_request(
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_faucet(signature_domain, &account_pk, signer_pk.as_ref(), nonce)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

fn parse_transfer_kind(kind: Option<&str>) -> PyResult<TransferKind> {
    match kind {
        Some("external") => Ok(TransferKind::External),
        Some("internal") | None => Ok(TransferKind::Internal),
        Some(other) => Err(PyValueError::new_err(format!(
            "Invalid transfer kind: {}",
            other
        ))),
    }
}

/// Prepare a sub-account removal for external signing
#[pyfunction]
#[pyo3(signature = (to_remove, signature_domain, account, signer=None, nonce=None))]
fn py_prepare_remove_sub_account(
    to_remove: &str,
    signature_domain: &str,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let target =
        Pubkey::from_base58(to_remove).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_remove_sub_account(
        target,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare a margin transfer for external signing
#[pyfunction]
#[pyo3(signature = (from_pubkey, to_pubkey, margin_amount, signature_domain, account, kind=None, signer=None, nonce=None))]
#[allow(clippy::too_many_arguments)]
fn py_prepare_transfer(
    from_pubkey: &str,
    to_pubkey: &str,
    margin_amount: f64,
    signature_domain: &str,
    account: &str,
    kind: Option<&str>,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let from =
        Pubkey::from_base58(from_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let to = Pubkey::from_base58(to_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let kind = parse_transfer_kind(kind)?;

    let transfer = Transfer {
        kind,
        from,
        to,
        margin_amount,
    };

    let prepared = prepare_transfer(
        transfer,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare a sub-account creation for external signing
#[pyfunction]
#[pyo3(signature = (name, signature_domain, account, margin_amount=None, signer=None, nonce=None))]
fn py_prepare_create_sub_account(
    name: String,
    signature_domain: &str,
    account: &str,
    margin_amount: Option<f64>,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let signature_domain = parse_signature_domain(signature_domain)?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let sub_account = CreateSubAccount {
        name,
        margin_amount,
    };

    let prepared = prepare_create_sub_account(
        sub_account,
        signature_domain,
        &account_pk,
        signer_pk.as_ref(),
        nonce,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Finalize a prepared message with a signature from an external wallet
///
/// Args:
///     prepared: PreparedMessage dict from prepare_* functions
///     signature: Base58-encoded signature from wallet
///
/// Returns:
///     SignedTransaction dict ready for API submission
///
/// Example:
///     prepared = prepare_order(order, "account_pubkey")
///     signature = wallet.sign_message(prepared["message_bytes"])
///     signed = finalize_transaction(prepared, signature)
#[pyfunction]
fn py_finalize_transaction(prepared: &Bound<'_, PyDict>, signature: &str) -> PyResult<PyObject> {
    let account: String = prepared
        .get_item("account")?
        .ok_or_else(|| PyValueError::new_err("Missing 'account'"))?
        .extract()?;
    let signer: String = prepared
        .get_item("signer")?
        .ok_or_else(|| PyValueError::new_err("Missing 'signer'"))?
        .extract()?;
    let nonce: u64 = prepared
        .get_item("nonce")?
        .ok_or_else(|| PyValueError::new_err("Missing 'nonce'"))?
        .extract()?;
    let actions = prepared
        .get_item("actions")?
        .ok_or_else(|| PyValueError::new_err("Missing 'actions'"))?;
    let order_id: Option<String> = prepared
        .get_item("order_id")?
        .map(|v| v.extract())
        .transpose()?;
    let order_ids: Option<Vec<String>> = prepared
        .get_item("order_ids")?
        .map(|v| v.extract())
        .transpose()?;

    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        dict.set_item("actions", actions)?;
        dict.set_item("nonce", nonce)?;
        dict.set_item("account", &account)?;
        dict.set_item("signer", &signer)?;
        dict.set_item("signature", signature)?;
        if let Some(order_id) = order_id {
            dict.set_item("order_id", &order_id)?;
        }
        if let Some(order_ids) = order_ids {
            dict.set_item("order_ids", &order_ids)?;
        }
        Ok(dict.into())
    })
}

// ============================================================================
// Module definition
// ============================================================================

/// High-performance transaction signing for BULK DEX
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyKeypair>()?;
    m.add_class::<PySigner>()?;
    m.add_function(wrap_pyfunction!(random_hash, m)?)?;
    m.add_function(wrap_pyfunction!(current_timestamp, m)?)?;
    m.add_function(wrap_pyfunction!(validate_pubkey, m)?)?;
    m.add_function(wrap_pyfunction!(validate_hash, m)?)?;
    m.add_function(wrap_pyfunction!(compute_order_id, m)?)?;
    m.add_function(wrap_pyfunction!(compute_order_id_from_order, m)?)?;
    // External wallet support
    m.add_function(wrap_pyfunction!(py_prepare_order, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_all_orders, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_order_group, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_agent_wallet_auth, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_approve_builder_code, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_update_liquidator_config, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_approve_commission_fee, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_revoke_builder_code, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_revoke_commission_fee, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_faucet_request, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_create_sub_account, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_remove_sub_account, m)?)?;
    m.add_function(wrap_pyfunction!(py_prepare_transfer, m)?)?;
    m.add_function(wrap_pyfunction!(py_finalize_transaction, m)?)?;
    Ok(())
}
