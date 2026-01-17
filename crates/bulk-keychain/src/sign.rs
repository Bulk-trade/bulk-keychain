//! Transaction signing
//!
//! High-performance Ed25519 signing with support for batch operations.

use crate::order_id::compute_order_item_id;
use crate::serialize::WincodeSerializer;
use crate::types::*;
use crate::{Error, Keypair, NonceManager, Result};
use ed25519_dalek::Signer as DalekSigner;
use rayon::prelude::*;
use serde_json::json;

/// Threshold for switching to parallel signing
const PARALLEL_THRESHOLD: usize = 10;

/// Transaction signer
///
/// Holds a keypair and provides methods for signing BULK transactions.
/// Designed for high-performance trading with minimal allocations.
pub struct Signer {
    keypair: Keypair,
    nonce_manager: Option<NonceManager>,
    /// Pre-allocated serializer for single-threaded use
    serializer: WincodeSerializer,
    /// Whether to compute order IDs (SHA256 of wincode bytes)
    compute_order_id: bool,
}

impl Signer {
    /// Create a new signer with the given keypair
    ///
    /// By default, order IDs are computed (SHA256 of wincode bytes).
    /// Use `without_order_id()` to disable if you don't need them.
    pub fn new(keypair: Keypair) -> Self {
        Self {
            keypair,
            nonce_manager: None,
            serializer: WincodeSerializer::new(),
            compute_order_id: true,
        }
    }

    /// Create a signer with automatic nonce management
    pub fn with_nonce_manager(keypair: Keypair, nonce_manager: NonceManager) -> Self {
        Self {
            keypair,
            nonce_manager: Some(nonce_manager),
            serializer: WincodeSerializer::new(),
            compute_order_id: true,
        }
    }

    /// Disable order ID computation for maximum performance
    ///
    /// Use this if you don't need to know the order ID before server response.
    /// Saves ~500ns per order (SHA256 cost).
    pub fn without_order_id(mut self) -> Self {
        self.compute_order_id = false;
        self
    }

    /// Enable order ID computation
    pub fn with_order_id(mut self) -> Self {
        self.compute_order_id = true;
        self
    }

    /// Check if order ID computation is enabled
    pub fn computes_order_id(&self) -> bool {
        self.compute_order_id
    }

    /// Get the signer's public key
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Get the next nonce (from manager or current timestamp)
    fn next_nonce(&self) -> u64 {
        self.nonce_manager
            .as_ref()
            .map(|m| m.next())
            .unwrap_or_else(crate::nonce::current_timestamp_millis)
    }

    // ========================================================================
    // Core signing method
    // ========================================================================

    /// Sign raw bytes and return base58-encoded signature
    pub fn sign_bytes(&self, message: &[u8]) -> String {
        let signature = self.keypair.signing_key().sign(message);
        bs58::encode(signature.to_bytes()).into_string()
    }

    /// Sign an action with the given nonce
    ///
    /// This is the low-level signing method. Most users should use the
    /// higher-level methods like `sign_order`, `sign_cancel`, etc.
    pub fn sign_action(
        &mut self,
        action: &Action,
        nonce: u64,
        account: &Pubkey,
    ) -> Result<SignedTransaction> {
        let signer_pubkey = self.keypair.pubkey();

        // Serialize for signing
        self.serializer.reset();
        self.serializer
            .serialize_for_signing(action, nonce, account, &signer_pubkey);

        // Compute order ID if enabled
        // Uses the new fixed-point algorithm for Order actions
        let order_id = if self.compute_order_id {
            self.compute_action_order_id(action, nonce, account)
        } else {
            None
        };

        // Sign
        let signature = self.sign_bytes(self.serializer.as_bytes());

        // Build JSON action
        let action_json = self.action_to_json(action, nonce);

        Ok(SignedTransaction {
            action: action_json,
            account: account.to_base58(),
            signer: signer_pubkey.to_base58(),
            signature,
            order_id,
        })
    }

    /// Compute order ID for an action using the new fixed-point algorithm
    fn compute_action_order_id(&self, action: &Action, nonce: u64, account: &Pubkey) -> Option<String> {
        match action {
            Action::Order { orders } if orders.len() == 1 => {
                // Single order: compute OID from the order itself
                compute_order_item_id(&orders[0], nonce, account).map(|h| h.to_base58())
            }
            _ => None, // Multi-order groups, Cancel, CancelAll, Faucet, etc. don't have OIDs
        }
    }

    /// Sign an action using own pubkey as account
    pub fn sign_action_self(&mut self, action: &Action, nonce: u64) -> Result<SignedTransaction> {
        let account = self.keypair.pubkey();
        self.sign_action(action, nonce, &account)
    }

    // ========================================================================
    // High-level signing methods - Simplified API
    // ========================================================================

    /// Sign a single order item
    ///
    /// This is a convenience method for the most common case.
    ///
    /// # Example
    /// ```rust,ignore
    /// let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
    /// let signed = signer.sign(order.into(), None)?;
    /// ```
    pub fn sign(&mut self, item: OrderItem, nonce: Option<u64>) -> Result<SignedTransaction> {
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let action = Action::Order { orders: vec![item] };
        self.sign_action_self(&action, nonce)
    }

    /// Sign multiple orders - each becomes its own transaction (parallel)
    ///
    /// This is the HFT-optimized method: each order gets its own transaction
    /// so they can be independently confirmed/rejected.
    ///
    /// Automatically parallelizes when > 10 orders.
    ///
    /// # Example
    /// ```rust,ignore
    /// let orders = vec![order1.into(), order2.into(), order3.into()];
    /// let signed_txs = signer.sign_all(orders, None)?; // Returns Vec<SignedTransaction>
    /// ```
    pub fn sign_all(
        &self,
        items: Vec<OrderItem>,
        base_nonce: Option<u64>,
    ) -> Result<Vec<SignedTransaction>> {
        if items.is_empty() {
            return Ok(vec![]);
        }

        let base = base_nonce.unwrap_or_else(crate::nonce::current_timestamp_millis);

        if items.len() < PARALLEL_THRESHOLD {
            // Sequential for small batches
            items
                .into_iter()
                .enumerate()
                .map(|(i, item)| self.sign_single_item(item, base + i as u64))
                .collect()
        } else {
            // Parallel for large batches
            items
                .into_par_iter()
                .enumerate()
                .map(|(i, item)| self.sign_single_item(item, base + i as u64))
                .collect()
        }
    }

    /// Sign multiple orders atomically in ONE transaction
    ///
    /// Use this for bracket orders (entry + stop loss + take profit)
    /// or other cases where all orders must succeed or fail together.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Bracket order: entry + stop loss + take profit
    /// let bracket = vec![entry.into(), stop_loss.into(), take_profit.into()];
    /// let signed = signer.sign_group(bracket, None)?; // Returns single SignedTransaction
    /// ```
    pub fn sign_group(
        &mut self,
        items: Vec<OrderItem>,
        nonce: Option<u64>,
    ) -> Result<SignedTransaction> {
        if items.is_empty() {
            return Err(Error::EmptyOrders);
        }
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let action = Action::Order { orders: items };
        self.sign_action_self(&action, nonce)
    }

    /// Internal: sign a single item (for parallel use)
    fn sign_single_item(&self, item: OrderItem, nonce: u64) -> Result<SignedTransaction> {
        let account = self.keypair.pubkey();
        let signer_pubkey = self.keypair.pubkey();

        // Compute order ID if enabled (before creating action to avoid clone)
        let order_id = if self.compute_order_id {
            compute_order_item_id(&item, nonce, &account).map(|h| h.to_base58())
        } else {
            None
        };

        let action = Action::Order { orders: vec![item] };

        let mut serializer = WincodeSerializer::new();
        serializer.serialize_for_signing(&action, nonce, &account, &signer_pubkey);

        let signature = self.sign_bytes(serializer.as_bytes());
        let action_json = self.action_to_json(&action, nonce);

        Ok(SignedTransaction {
            action: action_json,
            account: account.to_base58(),
            signer: signer_pubkey.to_base58(),
            signature,
            order_id,
        })
    }

    // ========================================================================
    // Legacy methods (kept for backward compatibility)
    // ========================================================================

    /// Sign order operations (place orders, cancel, cancel all)
    ///
    /// **Deprecated:** Use `sign()` for single orders, `sign_all()` for batches,
    /// or `sign_group()` for atomic multi-order transactions.
    #[deprecated(since = "0.2.0", note = "Use sign(), sign_all(), or sign_group() instead")]
    pub fn sign_order(
        &mut self,
        orders: Vec<OrderItem>,
        nonce: Option<u64>,
    ) -> Result<SignedTransaction> {
        self.sign_group(orders, nonce)
    }

    /// Sign a faucet request
    pub fn sign_faucet(&mut self, nonce: Option<u64>) -> Result<SignedTransaction> {
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let user = self.keypair.pubkey();
        let action = Action::Faucet(Faucet::new(user));
        self.sign_action_self(&action, nonce)
    }

    /// Sign agent wallet creation/deletion
    pub fn sign_agent_wallet(
        &mut self,
        agent: Pubkey,
        delete: bool,
        nonce: Option<u64>,
    ) -> Result<SignedTransaction> {
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let action = Action::AgentWalletCreation(AgentWallet {
            agent,
            delete,
        });
        self.sign_action_self(&action, nonce)
    }

    /// Sign user settings update
    pub fn sign_user_settings(
        &mut self,
        settings: UserSettings,
        nonce: Option<u64>,
    ) -> Result<SignedTransaction> {
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let action = Action::UpdateUserSettings(settings);
        self.sign_action_self(&action, nonce)
    }

    // ========================================================================
    // Legacy batch signing (kept for backward compatibility)
    // ========================================================================

    /// Sign multiple order batches in parallel
    ///
    /// **Deprecated:** Use `sign_all()` for simple batches (one order per tx),
    /// or call `sign_group()` multiple times if you need grouped batches.
    #[deprecated(since = "0.2.0", note = "Use sign_all() for simple batches")]
    pub fn sign_orders_batch(
        &self,
        order_batches: Vec<Vec<OrderItem>>,
        base_nonce: Option<u64>,
    ) -> Result<Vec<SignedTransaction>> {
        if order_batches.is_empty() {
            return Ok(vec![]);
        }

        let base = base_nonce.unwrap_or_else(crate::nonce::current_timestamp_millis);

        if order_batches.len() < PARALLEL_THRESHOLD {
            order_batches
                .into_iter()
                .enumerate()
                .map(|(i, orders)| self.sign_single_order_batch(orders, base + i as u64))
                .collect()
        } else {
            order_batches
                .into_par_iter()
                .enumerate()
                .map(|(i, orders)| self.sign_single_order_batch(orders, base + i as u64))
                .collect()
        }
    }

    /// Sign a single order batch (internal, creates new serializer)
    fn sign_single_order_batch(
        &self,
        orders: Vec<OrderItem>,
        nonce: u64,
    ) -> Result<SignedTransaction> {
        if orders.is_empty() {
            return Err(Error::EmptyOrders);
        }

        let account = self.keypair.pubkey();
        let signer_pubkey = self.keypair.pubkey();

        // Compute order ID if enabled (single order only)
        let order_id = if self.compute_order_id && orders.len() == 1 {
            compute_order_item_id(&orders[0], nonce, &account).map(|h| h.to_base58())
        } else {
            None
        };

        let action = Action::Order { orders };

        let mut serializer = WincodeSerializer::new();
        serializer.serialize_for_signing(&action, nonce, &account, &signer_pubkey);

        let signature = self.sign_bytes(serializer.as_bytes());
        let action_json = self.action_to_json(&action, nonce);

        Ok(SignedTransaction {
            action: action_json,
            account: account.to_base58(),
            signer: signer_pubkey.to_base58(),
            signature,
            order_id,
        })
    }

    // ========================================================================
    // JSON conversion
    // ========================================================================

    /// Convert an action to its JSON representation
    fn action_to_json(&self, action: &Action, nonce: u64) -> serde_json::Value {
        match action {
            Action::Order { orders } => {
                let orders_json: Vec<_> = orders.iter().map(|item| self.order_item_to_json(item)).collect();
                json!({
                    "type": "order",
                    "orders": orders_json,
                    "nonce": nonce
                })
            }
            Action::Faucet(faucet) => {
                let mut faucet_obj = json!({
                    "u": faucet.user.to_base58()
                });
                if let Some(amount) = faucet.amount {
                    faucet_obj["amount"] = json!(amount);
                }
                json!({
                    "type": "faucet",
                    "faucet": faucet_obj,
                    "nonce": nonce
                })
            }
            Action::AgentWalletCreation(agent) => {
                json!({
                    "type": "agentWalletCreation",
                    "agent": {
                        "a": agent.agent.to_base58(),
                        "d": agent.delete
                    },
                    "nonce": nonce
                })
            }
            Action::UpdateUserSettings(settings) => {
                let leverage_arr: Vec<_> = settings
                    .max_leverage
                    .iter()
                    .map(|(symbol, lev)| json!([symbol, lev]))
                    .collect();
                json!({
                    "type": "updateUserSettings",
                    "settings": {
                        "m": leverage_arr
                    },
                    "nonce": nonce
                })
            }
            Action::Oracle { oracles } => {
                let oracles_json: Vec<_> = oracles
                    .iter()
                    .map(|o| {
                        json!({
                            "t": o.timestamp,
                            "c": o.asset,
                            "px": o.price
                        })
                    })
                    .collect();
                json!({
                    "type": "oracle",
                    "oracles": oracles_json,
                    "nonce": nonce
                })
            }
        }
    }

    /// Convert an OrderItem to JSON
    fn order_item_to_json(&self, item: &OrderItem) -> serde_json::Value {
        match item {
            OrderItem::Order(order) => {
                let mut order_obj = json!({
                    "c": order.symbol,
                    "b": order.is_buy,
                    "px": order.price,
                    "sz": order.size,
                    "r": order.reduce_only,
                    "t": self.order_type_to_json(&order.order_type)
                });
                if let Some(ref cloid) = order.client_id {
                    order_obj["cloid"] = json!(cloid.to_base58());
                }
                json!({ "order": order_obj })
            }
            OrderItem::Cancel(cancel) => {
                json!({
                    "cancel": {
                        "c": cancel.symbol,
                        "oid": cancel.order_id.to_base58()
                    }
                })
            }
            OrderItem::CancelAll(cancel_all) => {
                json!({
                    "cancelAll": {
                        "c": cancel_all.symbols
                    }
                })
            }
        }
    }

    /// Convert OrderType to JSON
    fn order_type_to_json(&self, order_type: &OrderType) -> serde_json::Value {
        match order_type {
            OrderType::Limit { tif } => {
                let tif_str = match tif {
                    TimeInForce::Gtc => "GTC",
                    TimeInForce::Ioc => "IOC",
                    TimeInForce::Alo => "ALO",
                };
                json!({ "limit": { "tif": tif_str } })
            }
            OrderType::Trigger {
                is_market,
                trigger_px,
            } => {
                json!({
                    "trigger": {
                        "is_market": is_market,
                        "triggerPx": trigger_px
                    }
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // New simplified API tests
    // ========================================================================

    #[test]
    fn test_sign_single() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let result = signer.sign(order.into(), Some(1234567890));

        assert!(result.is_ok());
        let signed = result.unwrap();
        assert!(!signed.signature.is_empty());
        assert!(!signed.account.is_empty());
    }

    #[test]
    fn test_sign_all_parallel() {
        let keypair = Keypair::generate();
        let signer = Signer::new(keypair);

        // Create 20 independent orders
        let orders: Vec<OrderItem> = (0..20)
            .map(|i| Order::limit("BTC-USD", i % 2 == 0, 100000.0 + i as f64, 0.1, TimeInForce::Gtc).into())
            .collect();

        let results = signer.sign_all(orders, Some(1000000));
        assert!(results.is_ok());
        let signed = results.unwrap();
        assert_eq!(signed.len(), 20);

        // Each should have a unique nonce
        for (i, tx) in signed.iter().enumerate() {
            assert_eq!(tx.action["nonce"], 1000000 + i as u64);
        }
    }

    #[test]
    fn test_sign_group_atomic() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        // Bracket order: entry + stop loss + take profit
        let bracket: Vec<OrderItem> = vec![
            Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc).into(),
            Order::limit("BTC-USD", false, 99000.0, 0.1, TimeInForce::Gtc).into(),  // stop loss
            Order::limit("BTC-USD", false, 110000.0, 0.1, TimeInForce::Gtc).into(), // take profit
        ];

        let result = signer.sign_group(bracket, Some(1234567890));
        assert!(result.is_ok());

        let signed = result.unwrap();
        // All 3 orders in one transaction
        assert_eq!(signed.action["orders"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_sign_cancel() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        let order_id = Hash::random();
        let cancel = Cancel::new("BTC-USD", order_id);
        let result = signer.sign(cancel.into(), Some(1234567890));

        assert!(result.is_ok());
    }

    #[test]
    fn test_sign_cancel_all() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        let cancel_all = CancelAll::for_symbols(vec!["BTC-USD".to_string()]);
        let result = signer.sign(cancel_all.into(), Some(1234567890));

        assert!(result.is_ok());
    }

    #[test]
    fn test_sign_group_empty_error() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        let result = signer.sign_group(vec![], Some(1234567890));
        assert!(matches!(result, Err(Error::EmptyOrders)));
    }

    #[test]
    fn test_sign_all_empty_returns_empty() {
        let keypair = Keypair::generate();
        let signer = Signer::new(keypair);

        let result = signer.sign_all(vec![], Some(1234567890));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ========================================================================
    // Legacy API tests (deprecated but should still work)
    // ========================================================================

    #[test]
    #[allow(deprecated)]
    fn test_sign_order() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let result = signer.sign_order(vec![order.into()], Some(1234567890));

        assert!(result.is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_batch_signing() {
        let keypair = Keypair::generate();
        let signer = Signer::new(keypair);

        let batches: Vec<Vec<OrderItem>> = (0..20)
            .map(|i| {
                vec![Order::limit("BTC-USD", i % 2 == 0, 100000.0, 0.1, TimeInForce::Gtc).into()]
            })
            .collect();

        let results = signer.sign_orders_batch(batches, Some(1000000));
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 20);
    }

    // ========================================================================
    // Other action tests
    // ========================================================================

    #[test]
    fn test_sign_faucet() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        let result = signer.sign_faucet(Some(1234567890));
        assert!(result.is_ok());

        let signed = result.unwrap();
        assert_eq!(signed.action["type"], "faucet");
    }

    #[test]
    fn test_sign_agent_wallet() {
        let keypair = Keypair::generate();
        let agent_keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        let result = signer.sign_agent_wallet(agent_keypair.pubkey(), false, Some(1234567890));
        assert!(result.is_ok());
    }

    // ========================================================================
    // Order ID tests
    // ========================================================================

    #[test]
    fn test_order_id_computed_by_default() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);
        assert!(signer.computes_order_id());

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let signed = signer.sign(order.into(), Some(1234567890)).unwrap();

        // Order ID should be present (computed by default)
        assert!(signed.order_id.is_some());
        let order_id = signed.order_id.unwrap();
        // Should be a valid base58 string (32 bytes = ~44 chars)
        assert!(order_id.len() >= 40 && order_id.len() <= 50);
    }

    #[test]
    fn test_order_id_deterministic() {
        // Same inputs should produce same order ID
        let keypair = Keypair::generate();
        let mut signer1 = Signer::new(keypair.clone());
        let mut signer2 = Signer::new(keypair);

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let signed1 = signer1.sign(order.clone().into(), Some(1234567890)).unwrap();
        let signed2 = signer2.sign(order.into(), Some(1234567890)).unwrap();

        // Same order with same nonce = same order ID
        assert_eq!(signed1.order_id, signed2.order_id);
    }

    #[test]
    fn test_order_id_unique_per_nonce() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let signed1 = signer.sign(order.clone().into(), Some(1)).unwrap();
        let signed2 = signer.sign(order.into(), Some(2)).unwrap();

        // Different nonces = different order IDs
        assert_ne!(signed1.order_id, signed2.order_id);
    }

    #[test]
    fn test_order_id_disabled() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair).without_order_id();
        assert!(!signer.computes_order_id());

        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let signed = signer.sign(order.into(), Some(1234567890)).unwrap();

        // Order ID should be None when disabled
        assert!(signed.order_id.is_none());
    }

    #[test]
    fn test_order_id_in_batch() {
        let keypair = Keypair::generate();
        let signer = Signer::new(keypair);

        let orders: Vec<OrderItem> = (0..5)
            .map(|i| Order::limit("BTC-USD", true, 100000.0 + i as f64, 0.1, TimeInForce::Gtc).into())
            .collect();

        let signed = signer.sign_all(orders, Some(1000)).unwrap();
        
        // Each transaction should have a unique order ID
        let order_ids: Vec<_> = signed.iter().map(|t| t.order_id.clone().unwrap()).collect();
        let unique_ids: std::collections::HashSet<_> = order_ids.iter().collect();
        assert_eq!(order_ids.len(), unique_ids.len());
    }
}
