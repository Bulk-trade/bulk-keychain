//! Python bindings for BULK transaction signing
//!
//! This module provides high-performance Python bindings using PyO3.

use bulk_keychain::{
    compute_order_item_id, prepare_agent_wallet, prepare_all, prepare_faucet, prepare_group,
    prepare_message, Cancel, CancelAll, Hash, Keypair, Modify, NonceManager, NonceStrategy,
    OraclePrice, Order, OrderItem, OrderType, PreparedMessage, Pubkey, PythOraclePrice, Signer,
    TimeInForce, UserSettings,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

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
    fn new(keypair: &PyKeypair) -> Self {
        Self {
            inner: Signer::new(keypair.inner.clone()),
        }
    }

    /// Create a signer from base58-encoded secret key
    #[staticmethod]
    fn from_base58(s: &str) -> PyResult<Self> {
        let keypair = Keypair::from_base58(s).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: Signer::new(keypair),
        })
    }

    /// Create a signer with nonce management
    #[staticmethod]
    fn with_nonce_manager(keypair: &PyKeypair, strategy: &str) -> PyResult<Self> {
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
            inner: Signer::with_nonce_manager(keypair.inner.clone(), nonce_manager),
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
                order_type,
                client_id,
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
            order_type: OrderType::Limit { tif },
            client_id,
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

        return Ok(OrderItem::Order(Order {
            symbol,
            is_buy,
            price: 0.0,
            size,
            reduce_only,
            order_type: OrderType::Trigger {
                is_market: true,
                trigger_px: 0.0,
            },
            client_id: None,
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
#[pyo3(signature = (order, account, signer=None, nonce=None))]
fn py_prepare_order(
    order: &Bound<'_, PyAny>,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let order_item = parse_order_item(order)?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_message(order_item, &account_pk, signer_pk.as_ref(), nonce)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare multiple orders - each becomes its own transaction (parallel)
///
/// Optimized for HFT: each order gets independent confirmation/rejection.
///
/// Example:
///     prepared_list = prepare_all_orders([order1, order2], "account_pubkey")
#[pyfunction]
#[pyo3(signature = (orders, account, signer=None, base_nonce=None))]
fn py_prepare_all_orders(
    orders: &Bound<'_, PyList>,
    account: &str,
    signer: Option<&str>,
    base_nonce: Option<u64>,
) -> PyResult<PyObject> {
    let order_items: PyResult<Vec<OrderItem>> =
        orders.iter().map(|item| parse_order_item(&item)).collect();
    let order_items = order_items?;

    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_all(order_items, &account_pk, signer_pk.as_ref(), base_nonce)
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
#[pyo3(signature = (orders, account, signer=None, nonce=None))]
fn py_prepare_order_group(
    orders: &Bound<'_, PyList>,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let order_items: PyResult<Vec<OrderItem>> =
        orders.iter().map(|item| parse_order_item(&item)).collect();
    let order_items = order_items?;

    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_group(order_items, &account_pk, signer_pk.as_ref(), nonce)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare agent wallet creation for external signing
///
/// Example:
///     prepared = prepare_agent_wallet_auth(agent_pubkey, False, "account_pubkey")
#[pyfunction]
#[pyo3(signature = (agent_pubkey, delete, account, signer=None, nonce=None))]
fn py_prepare_agent_wallet_auth(
    agent_pubkey: &str,
    delete: bool,
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let agent =
        Pubkey::from_base58(agent_pubkey).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_agent_wallet(&agent, delete, &account_pk, signer_pk.as_ref(), nonce)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Python::with_gil(|py| prepared_to_py(py, &prepared))
}

/// Prepare faucet request for external signing
#[pyfunction]
#[pyo3(signature = (account, signer=None, nonce=None))]
fn py_prepare_faucet_request(
    account: &str,
    signer: Option<&str>,
    nonce: Option<u64>,
) -> PyResult<PyObject> {
    let account_pk =
        Pubkey::from_base58(account).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let signer_pk = signer
        .map(Pubkey::from_base58)
        .transpose()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let prepared = prepare_faucet(&account_pk, signer_pk.as_ref(), nonce)
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
    m.add_function(wrap_pyfunction!(py_prepare_faucet_request, m)?)?;
    m.add_function(wrap_pyfunction!(py_finalize_transaction, m)?)?;
    Ok(())
}
