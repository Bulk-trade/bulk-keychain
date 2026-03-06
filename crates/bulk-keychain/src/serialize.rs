//! Wincode binary serialization for BULK transaction signing.

use crate::types::*;
use crate::{Error, Result};

/// Pre-allocated buffer size for typical transactions.
const DEFAULT_BUFFER_SIZE: usize = 512;

/// Wincode serializer with a reusable pre-allocated buffer.
pub struct WincodeSerializer {
    buffer: Vec<u8>,
}

impl WincodeSerializer {
    /// Create a serializer with a default capacity.
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(DEFAULT_BUFFER_SIZE),
        }
    }

    /// Create a serializer with a custom capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Clear the internal buffer for reuse.
    #[inline]
    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    /// Borrow the serialized bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Consume the serializer and return the serialized bytes.
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    // ========================================================================
    // Primitive writers (little-endian)
    // ========================================================================

    #[inline]
    pub fn write_u32(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_u64(&mut self, value: u64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_i16(&mut self, value: i16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_f64(&mut self, value: f64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_bool(&mut self, value: bool) {
        self.buffer.push(if value { 1 } else { 0 });
    }

    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Write a string as `u64 length + utf8 bytes`.
    #[inline]
    pub fn write_string(&mut self, s: &str) {
        self.write_u64(s.len() as u64);
        self.buffer.extend_from_slice(s.as_bytes());
    }

    /// Write `Option<T>` as `bool + payload`.
    #[inline]
    pub fn write_option<T, F>(&mut self, opt: &Option<T>, write_fn: F)
    where
        F: FnOnce(&mut Self, &T),
    {
        match opt {
            Some(value) => {
                self.write_bool(true);
                write_fn(self, value);
            }
            None => self.write_bool(false),
        }
    }

    #[inline]
    pub fn write_pubkey(&mut self, pubkey: &Pubkey) {
        self.write_bytes(pubkey.as_bytes());
    }

    #[inline]
    pub fn write_hash(&mut self, hash: &Hash) {
        self.write_bytes(hash.as_bytes());
    }

    // ========================================================================
    // Action serializers (wire format from bulk-api http-readme.md)
    // ========================================================================

    /// Serialize one wire action (already flattened).
    pub fn write_order_item_action(&mut self, item: &OrderItem) -> Result<()> {
        match item {
            OrderItem::Order(order) => match &order.order_type {
                OrderType::Limit { tif } => {
                    // l => discriminant 1
                    self.write_u32(1);
                    self.write_string(&order.symbol);
                    self.write_bool(order.is_buy);
                    self.write_f64(order.price);
                    self.write_f64(order.size);
                    self.write_u32(tif.discriminant());
                    self.write_bool(order.reduce_only);
                    Ok(())
                }
                OrderType::Trigger {
                    is_market,
                    trigger_px: _,
                } => {
                    if !is_market {
                        return Err(Error::InvalidOrder(
                            "trigger orders are not supported by BULK API; use limit or market"
                                .to_string(),
                        ));
                    }
                    // m => discriminant 0
                    self.write_u32(0);
                    self.write_string(&order.symbol);
                    self.write_bool(order.is_buy);
                    self.write_f64(order.size);
                    self.write_bool(order.reduce_only);
                    Ok(())
                }
            },
            OrderItem::Modify(modify) => {
                // mod => discriminant 2
                self.write_u32(2);
                self.write_hash(&modify.order_id);
                self.write_string(&modify.symbol);
                self.write_f64(modify.amount);
                Ok(())
            }
            OrderItem::Cancel(cancel) => {
                // cx => discriminant 3
                self.write_u32(3);
                self.write_string(&cancel.symbol);
                self.write_hash(&cancel.order_id);
                Ok(())
            }
            OrderItem::CancelAll(cancel_all) => {
                // cxa => discriminant 4
                self.write_u32(4);
                self.write_u64(cancel_all.symbols.len() as u64);
                for symbol in &cancel_all.symbols {
                    self.write_string(symbol);
                }
                Ok(())
            }
        }
    }

    /// Serialize a compact `px` action (discriminant 5).
    pub fn write_price_action(&mut self, price: &OraclePrice) {
        self.write_u32(5);
        self.write_u64(price.timestamp);
        self.write_string(&price.asset);
        self.write_f64(price.price);
    }

    /// Serialize a compact `o` action (discriminant 6).
    pub fn write_pyth_oracle_action(&mut self, oracles: &[PythOraclePrice]) {
        self.write_u32(6);
        self.write_u64(oracles.len() as u64);
        for oracle in oracles {
            self.write_u64(oracle.timestamp);
            self.write_u64(oracle.feed_index);
            self.write_u64(oracle.price);
            self.write_i16(oracle.exponent);
        }
    }

    /// Serialize a compact `faucet` action (discriminant 7).
    pub fn write_faucet_action(&mut self, faucet: &Faucet) {
        self.write_u32(7);
        self.write_pubkey(&faucet.user);
        self.write_option(&faucet.amount, |s, &a| s.write_f64(a));
    }

    /// Serialize a compact `agentWalletCreation` action (discriminant 8).
    pub fn write_agent_wallet_action(&mut self, agent: &AgentWallet) {
        self.write_u32(8);
        self.write_pubkey(&agent.agent);
        self.write_bool(agent.delete);
    }

    /// Serialize a compact `updateUserSettings` action (discriminant 9).
    pub fn write_user_settings_action(&mut self, settings: &UserSettings) {
        self.write_u32(9);
        self.write_u64(settings.max_leverage.len() as u64);
        for (symbol, leverage) in &settings.max_leverage {
            self.write_string(symbol);
            self.write_f64(*leverage);
        }
    }

    /// Serialize a compact `whitelistFaucet` action (discriminant 10).
    pub fn write_whitelist_faucet_action(&mut self, action: &WhitelistFaucet) {
        self.write_u32(10);
        self.write_pubkey(&action.target);
        self.write_bool(action.whitelist);
    }

    /// Serialize a complete transaction message for signing:
    /// `actions + nonce + account + signer`.
    pub fn serialize_for_signing(
        &mut self,
        action: &Action,
        nonce: u64,
        account: &Pubkey,
        signer: &Pubkey,
    ) -> Result<()> {
        // 1) action count
        let action_count = match action {
            Action::Order { orders } => orders.len() as u64,
            Action::Oracle { oracles } => oracles.len() as u64,
            Action::PythOracle { .. } => 1,
            _ => 1,
        };

        if action_count == 0 {
            return Err(Error::EmptyOrders);
        }

        self.write_u64(action_count);

        // 2) action bodies
        match action {
            Action::Order { orders } => {
                for item in orders {
                    self.write_order_item_action(item)?;
                }
            }
            Action::Oracle { oracles } => {
                for price in oracles {
                    self.write_price_action(price);
                }
            }
            Action::PythOracle { oracles } => self.write_pyth_oracle_action(oracles),
            Action::Faucet(faucet) => self.write_faucet_action(faucet),
            Action::UpdateUserSettings(settings) => self.write_user_settings_action(settings),
            Action::AgentWalletCreation(agent) => self.write_agent_wallet_action(agent),
            Action::WhitelistFaucet(action) => self.write_whitelist_faucet_action(action),
        }

        // 3) trailing transaction fields
        self.write_u64(nonce);
        self.write_pubkey(account);
        self.write_pubkey(signer);

        Ok(())
    }
}

impl Default for WincodeSerializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_primitives() {
        let mut s = WincodeSerializer::new();

        s.write_u32(42);
        assert_eq!(s.as_bytes(), &[42, 0, 0, 0]);

        s.reset();
        s.write_u64(1234567890);
        assert_eq!(s.as_bytes(), &1234567890u64.to_le_bytes());

        s.reset();
        s.write_bool(true);
        s.write_bool(false);
        assert_eq!(s.as_bytes(), &[1, 0]);

        s.reset();
        s.write_string("BTC-USD");
        assert_eq!(s.as_bytes().len(), 8 + 7);
    }

    #[test]
    fn test_serialize_limit_order_action() {
        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let action = Action::Order {
            orders: vec![order.into()],
        };
        let account = Pubkey::from_bytes([1u8; 32]);

        let mut s = WincodeSerializer::new();
        s.serialize_for_signing(&action, 123, &account, &account)
            .unwrap();

        assert!(!s.as_bytes().is_empty());
        // first 8 bytes are action count = 1
        assert_eq!(&s.as_bytes()[..8], &1u64.to_le_bytes());
    }
}
