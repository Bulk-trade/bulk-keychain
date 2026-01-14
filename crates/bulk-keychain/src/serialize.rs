//! Wincode binary serialization
//!
//! The BULK exchange uses a custom binary format called "wincode" for signing.
//! This module implements zero-copy serialization optimized for signing performance.

use crate::types::*;

/// Pre-allocated buffer size for typical transactions
const DEFAULT_BUFFER_SIZE: usize = 512;

/// Wincode serializer with pre-allocated buffer
pub struct WincodeSerializer {
    buffer: Vec<u8>,
}

impl WincodeSerializer {
    /// Create a new serializer with default buffer size
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(DEFAULT_BUFFER_SIZE),
        }
    }

    /// Create a new serializer with specific capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Reset buffer for reuse (avoids reallocation)
    #[inline]
    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    /// Get the serialized bytes
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Consume and return the buffer
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    // ========================================================================
    // Primitive writers (all little-endian)
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

    /// Write a string (u64 length prefix + UTF-8 bytes)
    #[inline]
    pub fn write_string(&mut self, s: &str) {
        self.write_u64(s.len() as u64);
        self.buffer.extend_from_slice(s.as_bytes());
    }

    /// Write Option<T> (1 byte discriminant + T if Some)
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
            None => {
                self.write_bool(false);
            }
        }
    }

    // ========================================================================
    // Type serializers
    // ========================================================================

    /// Write a Pubkey (raw 32 bytes)
    #[inline]
    pub fn write_pubkey(&mut self, pubkey: &Pubkey) {
        self.write_bytes(pubkey.as_bytes());
    }

    /// Write a Hash (raw 32 bytes)
    #[inline]
    pub fn write_hash(&mut self, hash: &Hash) {
        self.write_bytes(hash.as_bytes());
    }

    /// Write an Order
    pub fn write_order(&mut self, order: &Order) {
        // symbol (string)
        self.write_string(&order.symbol);
        // is_buy (bool)
        self.write_bool(order.is_buy);
        // price (f64)
        self.write_f64(order.price);
        // size (f64)
        self.write_f64(order.size);
        // reduce_only (bool)
        self.write_bool(order.reduce_only);

        // order_type (discriminant + data)
        self.write_u32(order.order_type.discriminant());
        match &order.order_type {
            OrderType::Limit { tif } => {
                self.write_u32(tif.discriminant());
            }
            OrderType::Trigger {
                is_market,
                trigger_px,
            } => {
                self.write_bool(*is_market);
                self.write_f64(*trigger_px);
            }
        }

        // client_id (Option<Hash>)
        self.write_option(&order.client_id, |s, h| s.write_hash(h));
    }

    /// Write a Cancel
    pub fn write_cancel(&mut self, cancel: &Cancel) {
        self.write_string(&cancel.symbol);
        self.write_hash(&cancel.order_id);
    }

    /// Write a CancelAll
    pub fn write_cancel_all(&mut self, cancel_all: &CancelAll) {
        self.write_u64(cancel_all.symbols.len() as u64);
        for symbol in &cancel_all.symbols {
            self.write_string(symbol);
        }
    }

    /// Write an OrderItem
    pub fn write_order_item(&mut self, item: &OrderItem) {
        self.write_u32(item.discriminant());
        match item {
            OrderItem::Order(order) => self.write_order(order),
            OrderItem::Cancel(cancel) => self.write_cancel(cancel),
            OrderItem::CancelAll(cancel_all) => self.write_cancel_all(cancel_all),
        }
    }

    /// Write a Faucet
    pub fn write_faucet(&mut self, faucet: &Faucet) {
        self.write_pubkey(&faucet.user);
        self.write_option(&faucet.amount, |s, &a| s.write_f64(a));
    }

    /// Write an AgentWallet
    pub fn write_agent_wallet(&mut self, agent: &AgentWallet) {
        self.write_pubkey(&agent.agent);
        self.write_bool(agent.delete);
    }

    /// Write UserSettings
    pub fn write_user_settings(&mut self, settings: &UserSettings) {
        self.write_u64(settings.max_leverage.len() as u64);
        for (symbol, leverage) in &settings.max_leverage {
            self.write_string(symbol);
            self.write_f64(*leverage);
        }
    }

    /// Write Oracle prices
    pub fn write_oracles(&mut self, oracles: &[OraclePrice]) {
        self.write_u64(oracles.len() as u64);
        for oracle in oracles {
            self.write_u64(oracle.timestamp);
            self.write_string(&oracle.asset);
            self.write_f64(oracle.price);
        }
    }

    /// Serialize a complete transaction for signing
    ///
    /// Format: action_discriminant(u32) + action_data + nonce(u64) + account(32) + signer(32)
    pub fn serialize_for_signing(
        &mut self,
        action: &Action,
        nonce: u64,
        account: &Pubkey,
        signer: &Pubkey,
    ) {
        // 1. Action discriminant
        self.write_u32(action.discriminant());

        // 2. Action-specific data
        match action {
            Action::Order { orders } => {
                self.write_u64(orders.len() as u64);
                for item in orders {
                    self.write_order_item(item);
                }
            }
            Action::Oracle { oracles } => {
                self.write_oracles(oracles);
            }
            Action::Faucet(faucet) => {
                self.write_faucet(faucet);
            }
            Action::UpdateUserSettings(settings) => {
                self.write_user_settings(settings);
            }
            Action::AgentWalletCreation(agent) => {
                self.write_agent_wallet(agent);
            }
        }

        // 3. Nonce
        self.write_u64(nonce);

        // 4. Account pubkey (raw 32 bytes)
        self.write_pubkey(account);

        // 5. Signer pubkey (raw 32 bytes)
        self.write_pubkey(signer);
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
        assert_eq!(s.as_bytes().len(), 8 + 7); // u64 length + "BTC-USD"
    }

    #[test]
    fn test_serialize_order() {
        let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
        let mut s = WincodeSerializer::new();
        s.write_order(&order);

        // Should not panic and produce some bytes
        assert!(!s.as_bytes().is_empty());
    }
}
