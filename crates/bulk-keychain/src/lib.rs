//! # bulk-keychain
//!
//! A simple high perf signing lib for BULK txns.
//!
//! This crate provides the core functionality for signing transactions
//! on BULK. Designed for low-latency, high-throughput trading systems.
//!
//! ## Features
//!
//! - **Fast signing**: Ed25519 signatures with SIMD optimizations
//! - **Batch signing**: Sign multiple transactions in parallel using Rayon
//! - **Zero-copy serialization**: Minimal allocations in hot paths
//! - **Type-safe**: Rust's type system prevents malformed transactions
//!
//! ## Quick Start
//!
//! ```rust
//! use bulk_keychain::{Keypair, Signer, Order, OrderType, TimeInForce};
//!
//! // Generate a new keypair
//! let keypair = Keypair::generate();
//!
//! // Create an order
//! let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
//!
//! // Sign the transaction
//! let mut signer = Signer::new(keypair);
//! let signed_tx = signer.sign_order(vec![order.into()], None).unwrap();
//! ```
//!
//! ## Batch Signing
//!
//! For high-frequency trading, use batch signing to maximize throughput:
//!
//! ```rust
//! use bulk_keychain::{Keypair, Signer, Order, TimeInForce};
//!
//! let keypair = Keypair::generate();
//! let signer = Signer::new(keypair);
//!
//! // Create many orders
//! let orders: Vec<_> = (0..1000)
//!     .map(|i| vec![Order::limit("BTC-USD", i % 2 == 0, 100000.0 + i as f64, 0.1, TimeInForce::Gtc).into()])
//!     .collect();
//!
//! // Sign all at once - automatically uses parallel signing for large batches
//! let signed_txs = signer.sign_orders_batch(orders, None).unwrap();
//! ```

mod error;
mod keypair;
pub mod nonce;
pub mod order_id;
pub mod prepare;
pub mod serialize;
mod sign;
pub mod types;

pub use error::{Error, Result};
pub use keypair::Keypair;
pub use nonce::{NonceManager, NonceStrategy};
pub use order_id::{
    compute_limit_order_id, compute_market_order_id, compute_order_id,
    compute_order_item_id,
};
pub use prepare::{
    finalize_all, finalize_transaction, finalize_transaction_bytes,
    prepare_action, prepare_agent_wallet, prepare_all, prepare_faucet,
    prepare_group, prepare_message, prepare_user_settings, PreparedMessage,
};
pub use sign::Signer;
pub use types::*;

/// Re-export for convenience
pub use bs58;
pub use ed25519_dalek;
