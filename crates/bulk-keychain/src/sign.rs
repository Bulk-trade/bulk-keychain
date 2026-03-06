//! Transaction signing.

use crate::order_id::compute_order_item_id;
use crate::serialize::WincodeSerializer;
use crate::types::*;
use crate::{Error, Keypair, NonceManager, Result};
use ed25519_dalek::Signer as DalekSigner;
use rayon::prelude::*;
use serde_json::json;

/// Threshold for switching to parallel signing.
const PARALLEL_THRESHOLD: usize = 10;

/// High-performance signer.
pub struct Signer {
    keypair: Keypair,
    nonce_manager: Option<NonceManager>,
    serializer: WincodeSerializer,
    compute_order_id: bool,
    compute_batch_order_ids: bool,
}

impl Signer {
    /// Create a signer.
    pub fn new(keypair: Keypair) -> Self {
        Self {
            keypair,
            nonce_manager: None,
            serializer: WincodeSerializer::new(),
            compute_order_id: true,
            compute_batch_order_ids: false,
        }
    }

    /// Create a signer with nonce management.
    pub fn with_nonce_manager(keypair: Keypair, nonce_manager: NonceManager) -> Self {
        Self {
            keypair,
            nonce_manager: Some(nonce_manager),
            serializer: WincodeSerializer::new(),
            compute_order_id: true,
            compute_batch_order_ids: false,
        }
    }

    /// Disable optional pre-computed order ID generation.
    pub fn without_order_id(mut self) -> Self {
        self.compute_order_id = false;
        self
    }

    /// Enable optional pre-computed order ID generation.
    pub fn with_order_id(mut self) -> Self {
        self.compute_order_id = true;
        self
    }

    /// Check whether order ID generation is enabled.
    pub fn computes_order_id(&self) -> bool {
        self.compute_order_id
    }

    /// Set whether single-order ID generation is enabled.
    pub fn set_order_id(&mut self, enabled: bool) {
        self.compute_order_id = enabled;
    }

    /// Enable optional pre-computed batch order IDs for multi-order transactions.
    ///
    /// When enabled, `sign_group()` and grouped legacy batch methods include `order_ids`.
    /// Default is disabled to avoid extra hashing work in hot paths.
    pub fn with_batch_order_ids(mut self) -> Self {
        self.compute_batch_order_ids = true;
        self
    }

    /// Disable optional pre-computed batch order IDs.
    pub fn without_batch_order_ids(mut self) -> Self {
        self.compute_batch_order_ids = false;
        self
    }

    /// Check whether batch order ID generation is enabled.
    pub fn computes_batch_order_ids(&self) -> bool {
        self.compute_batch_order_ids
    }

    /// Set whether batch order ID generation is enabled.
    pub fn set_batch_order_ids(&mut self, enabled: bool) {
        self.compute_batch_order_ids = enabled;
    }

    /// Get signer pubkey.
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Sign raw bytes and return base58 signature.
    pub fn sign_bytes(&self, message: &[u8]) -> String {
        let signature = self.keypair.signing_key().sign(message);
        bs58::encode(signature.to_bytes()).into_string()
    }

    fn next_nonce(&self) -> u64 {
        self.nonce_manager
            .as_ref()
            .map(|m| m.next())
            .unwrap_or_else(crate::nonce::current_timestamp_millis)
    }

    /// Low-level signing entrypoint.
    pub fn sign_action(
        &mut self,
        action: &Action,
        nonce: u64,
        account: &Pubkey,
    ) -> Result<SignedTransaction> {
        let signer_pubkey = self.keypair.pubkey();

        self.serializer.reset();
        self.serializer
            .serialize_for_signing(action, nonce, account, &signer_pubkey)?;

        let order_id = if self.compute_order_id {
            self.compute_action_order_id(action, nonce, account)
        } else {
            None
        };
        let order_ids = if self.compute_batch_order_ids {
            self.compute_action_order_ids(action, nonce, account)
        } else {
            None
        };

        let signature = self.sign_bytes(self.serializer.as_bytes());
        let actions = self.action_to_json(action)?;

        Ok(SignedTransaction {
            actions,
            nonce,
            account: account.to_base58(),
            signer: signer_pubkey.to_base58(),
            signature,
            order_id,
            order_ids,
        })
    }

    /// Sign using signer pubkey as account.
    pub fn sign_action_self(&mut self, action: &Action, nonce: u64) -> Result<SignedTransaction> {
        let account = self.keypair.pubkey();
        self.sign_action(action, nonce, &account)
    }

    /// Sign a single order item.
    pub fn sign(&mut self, item: OrderItem, nonce: Option<u64>) -> Result<SignedTransaction> {
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let action = Action::Order { orders: vec![item] };
        self.sign_action_self(&action, nonce)
    }

    /// Sign multiple independent items in parallel.
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
            items
                .into_iter()
                .enumerate()
                .map(|(i, item)| self.sign_single_item(item, base + i as u64))
                .collect()
        } else {
            items
                .into_par_iter()
                .enumerate()
                .map(|(i, item)| self.sign_single_item(item, base + i as u64))
                .collect()
        }
    }

    /// Sign multiple items atomically as one transaction.
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

    fn sign_single_item(&self, item: OrderItem, nonce: u64) -> Result<SignedTransaction> {
        let account = self.keypair.pubkey();
        let signer_pubkey = self.keypair.pubkey();
        let order_id = if self.compute_order_id {
            compute_order_item_id(&item, nonce, &account).map(|h| h.to_base58())
        } else {
            None
        };

        let action = Action::Order { orders: vec![item] };
        let mut serializer = WincodeSerializer::new();
        serializer.serialize_for_signing(&action, nonce, &account, &signer_pubkey)?;

        let signature = self.sign_bytes(serializer.as_bytes());
        let actions = self.action_to_json(&action)?;

        Ok(SignedTransaction {
            actions,
            nonce,
            account: account.to_base58(),
            signer: signer_pubkey.to_base58(),
            signature,
            order_id,
            order_ids: None,
        })
    }

    /// Sign a faucet action.
    pub fn sign_faucet(&mut self, nonce: Option<u64>) -> Result<SignedTransaction> {
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let action = Action::Faucet(Faucet::new(self.keypair.pubkey()));
        self.sign_action_self(&action, nonce)
    }

    /// Sign agent wallet creation/deletion.
    pub fn sign_agent_wallet(
        &mut self,
        agent: Pubkey,
        delete: bool,
        nonce: Option<u64>,
    ) -> Result<SignedTransaction> {
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let action = Action::AgentWalletCreation(AgentWallet { agent, delete });
        self.sign_action_self(&action, nonce)
    }

    /// Sign user settings update.
    pub fn sign_user_settings(
        &mut self,
        settings: UserSettings,
        nonce: Option<u64>,
    ) -> Result<SignedTransaction> {
        let nonce = nonce.unwrap_or_else(|| self.next_nonce());
        let action = Action::UpdateUserSettings(settings);
        self.sign_action_self(&action, nonce)
    }

    /// Deprecated compatibility alias.
    #[deprecated(
        since = "0.2.0",
        note = "Use sign(), sign_all(), or sign_group() instead"
    )]
    pub fn sign_order(
        &mut self,
        orders: Vec<OrderItem>,
        nonce: Option<u64>,
    ) -> Result<SignedTransaction> {
        self.sign_group(orders, nonce)
    }

    /// Deprecated compatibility alias.
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
        let order_id = if self.compute_order_id && orders.len() == 1 {
            compute_order_item_id(&orders[0], nonce, &account).map(|h| h.to_base58())
        } else {
            None
        };
        let order_ids = if self.compute_batch_order_ids && orders.len() > 1 {
            let ids: Vec<String> = orders
                .iter()
                .filter_map(|item| {
                    compute_order_item_id(item, nonce, &account).map(|h| h.to_base58())
                })
                .collect();
            if ids.is_empty() {
                None
            } else {
                Some(ids)
            }
        } else {
            None
        };

        let action = Action::Order { orders };
        let mut serializer = WincodeSerializer::new();
        serializer.serialize_for_signing(&action, nonce, &account, &signer_pubkey)?;
        let signature = self.sign_bytes(serializer.as_bytes());
        let actions = self.action_to_json(&action)?;

        Ok(SignedTransaction {
            actions,
            nonce,
            account: account.to_base58(),
            signer: signer_pubkey.to_base58(),
            signature,
            order_id,
            order_ids,
        })
    }

    fn compute_action_order_id(
        &self,
        action: &Action,
        nonce: u64,
        account: &Pubkey,
    ) -> Option<String> {
        match action {
            Action::Order { orders } if orders.len() == 1 => {
                compute_order_item_id(&orders[0], nonce, account).map(|h| h.to_base58())
            }
            _ => None,
        }
    }

    fn compute_action_order_ids(
        &self,
        action: &Action,
        nonce: u64,
        account: &Pubkey,
    ) -> Option<Vec<String>> {
        match action {
            Action::Order { orders } if orders.len() > 1 => {
                let ids: Vec<String> = orders
                    .iter()
                    .filter_map(|item| {
                        compute_order_item_id(item, nonce, account).map(|h| h.to_base58())
                    })
                    .collect();
                if ids.is_empty() {
                    None
                } else {
                    Some(ids)
                }
            }
            _ => None,
        }
    }

    fn action_to_json(&self, action: &Action) -> Result<Vec<serde_json::Value>> {
        match action {
            Action::Order { orders } => orders
                .iter()
                .map(|item| self.order_item_to_json(item))
                .collect(),
            Action::Faucet(faucet) => {
                let mut faucet_obj = json!({ "u": faucet.user.to_base58() });
                if let Some(amount) = faucet.amount {
                    faucet_obj["amount"] = json!(amount);
                }
                Ok(vec![json!({ "faucet": faucet_obj })])
            }
            Action::AgentWalletCreation(agent) => Ok(vec![json!({
                "agentWalletCreation": {
                    "a": agent.agent.to_base58(),
                    "d": agent.delete
                }
            })]),
            Action::UpdateUserSettings(settings) => {
                let leverage: Vec<_> = settings
                    .max_leverage
                    .iter()
                    .map(|(symbol, lev)| json!([symbol, lev]))
                    .collect();
                Ok(vec![json!({ "updateUserSettings": { "m": leverage } })])
            }
            Action::Oracle { oracles } => Ok(oracles
                .iter()
                .map(|o| {
                    json!({
                        "px": {
                            "t": o.timestamp,
                            "c": o.asset,
                            "px": o.price
                        }
                    })
                })
                .collect()),
            Action::WhitelistFaucet(action) => Ok(vec![json!({
                "whitelistFaucet": {
                    "target": action.target.to_base58(),
                    "whitelist": action.whitelist
                }
            })]),
        }
    }

    fn order_item_to_json(&self, item: &OrderItem) -> Result<serde_json::Value> {
        match item {
            OrderItem::Order(order) => match &order.order_type {
                OrderType::Limit { tif } => {
                    let tif_str = match tif {
                        TimeInForce::Gtc => "GTC",
                        TimeInForce::Ioc => "IOC",
                        TimeInForce::Alo => "ALO",
                    };
                    Ok(json!({
                        "l": {
                            "c": order.symbol,
                            "b": order.is_buy,
                            "px": order.price,
                            "sz": order.size,
                            "tif": tif_str,
                            "r": order.reduce_only
                        }
                    }))
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
                    Ok(json!({
                        "m": {
                            "c": order.symbol,
                            "b": order.is_buy,
                            "sz": order.size,
                            "r": order.reduce_only
                        }
                    }))
                }
            },
            OrderItem::Modify(modify) => Ok(json!({
                "mod": {
                    "oid": modify.order_id.to_base58(),
                    "symbol": modify.symbol,
                    "amount": modify.amount
                }
            })),
            OrderItem::Cancel(cancel) => Ok(json!({
                "cx": {
                    "c": cancel.symbol,
                    "oid": cancel.order_id.to_base58()
                }
            })),
            OrderItem::CancelAll(cancel_all) => Ok(json!({
                "cxa": {
                    "c": cancel_all.symbols
                }
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_single() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);
        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let signed = signer.sign(order.into(), Some(1234567890)).unwrap();

        assert_eq!(signed.nonce, 1234567890);
        assert_eq!(signed.actions.len(), 1);
        assert!(signed.actions[0].get("l").is_some());
        assert!(!signed.signature.is_empty());
    }

    #[test]
    fn test_sign_all_parallel_nonce_increment() {
        let keypair = Keypair::generate();
        let signer = Signer::new(keypair);
        let orders: Vec<OrderItem> = (0..20)
            .map(|i| {
                Order::limit(
                    "BTC-USD",
                    i % 2 == 0,
                    100000.0 + i as f64,
                    0.1,
                    TimeInForce::Gtc,
                )
                .into()
            })
            .collect();

        let signed = signer.sign_all(orders, Some(1000000)).unwrap();
        assert_eq!(signed.len(), 20);
        for (i, tx) in signed.iter().enumerate() {
            assert_eq!(tx.nonce, 1000000 + i as u64);
        }
    }

    #[test]
    fn test_sign_group_atomic() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);
        let bracket: Vec<OrderItem> = vec![
            Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc).into(),
            Order::limit("BTC-USD", false, 99000.0, 0.1, TimeInForce::Gtc).into(),
            Order::limit("BTC-USD", false, 110000.0, 0.1, TimeInForce::Gtc).into(),
        ];

        let signed = signer.sign_group(bracket, Some(1234567890)).unwrap();
        assert_eq!(signed.actions.len(), 3);
        assert_eq!(signed.nonce, 1234567890);
        assert!(signed.order_ids.is_none());
    }

    #[test]
    fn test_sign_faucet() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);
        let signed = signer.sign_faucet(Some(1234567890)).unwrap();
        assert_eq!(signed.actions.len(), 1);
        assert!(signed.actions[0].get("faucet").is_some());
    }

    #[test]
    fn test_sign_agent_wallet() {
        let keypair = Keypair::generate();
        let agent_keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);
        let signed = signer
            .sign_agent_wallet(agent_keypair.pubkey(), false, Some(1234567890))
            .unwrap();
        assert!(signed.actions[0].get("agentWalletCreation").is_some());
    }

    #[test]
    fn test_sign_group_empty_error() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);
        let result = signer.sign_group(vec![], Some(1234567890));
        assert!(matches!(result, Err(Error::EmptyOrders)));
    }

    #[test]
    fn test_order_id_computed_by_default() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair);
        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let signed = signer.sign(order.into(), Some(1234567890)).unwrap();
        assert!(signed.order_id.is_some());
    }

    #[test]
    fn test_order_id_disabled() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair).without_order_id();
        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let signed = signer.sign(order.into(), Some(1234567890)).unwrap();
        assert!(signed.order_id.is_none());
    }

    #[test]
    fn test_batch_order_ids_optional_enabled() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair).with_batch_order_ids();
        assert!(signer.computes_batch_order_ids());

        let bracket: Vec<OrderItem> = vec![
            Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc).into(),
            Order::limit("BTC-USD", false, 99000.0, 0.1, TimeInForce::Gtc).into(),
            Order::limit("BTC-USD", false, 110000.0, 0.1, TimeInForce::Gtc).into(),
        ];

        let signed = signer.sign_group(bracket, Some(1234567890)).unwrap();
        let ids = signed.order_ids.expect("order_ids should be present");
        assert_eq!(ids.len(), 3);
        assert!(signed.order_id.is_none());
    }

    #[test]
    #[allow(deprecated)]
    fn test_legacy_methods_still_work() {
        let keypair = Keypair::generate();
        let mut signer = Signer::new(keypair.clone());
        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let one = signer
            .sign_order(vec![order.clone().into()], Some(100))
            .unwrap();
        assert_eq!(one.actions.len(), 1);

        let signer = Signer::new(keypair);
        let batches = vec![vec![order.into()]];
        let many = signer.sign_orders_batch(batches, Some(200)).unwrap();
        assert_eq!(many.len(), 1);
    }
}
