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
//! use bulk_keychain::{Keypair, Signer, Order, TimeInForce};
//!
//! // Generate a new keypair
//! let keypair = Keypair::generate();
//!
//! // Create an order
//! let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
//!
//! // Sign the transaction
//! let mut signer = Signer::new(keypair);
//! let signed_tx = signer.sign(order.into(), None).unwrap();
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
//! let mut signer = Signer::new(keypair);
//!
//! // Create many independent orders
//! let orders: Vec<_> = (0..1000)
//!     .map(|i| Order::limit("BTC-USD", i % 2 == 0, 100000.0 + i as f64, 0.1, TimeInForce::Gtc).into())
//!     .collect();
//!
//! // Sign all at once - automatically uses parallel signing for large batches
//! let signed_txs = signer.sign_all(orders, None).unwrap();
//! ```

mod error;
mod keypair;
pub mod nonce;
pub mod order_id;
pub mod prepare;
mod sdk_compat;
mod sign;
pub mod types;

pub use error::{Error, Result};
pub use keypair::Keypair;
pub use nonce::{NonceManager, NonceStrategy};
pub use order_id::{
    compute_limit_order_id, compute_market_order_id, compute_order_id, compute_order_item_id,
};
pub use prepare::{
    finalize_all, finalize_transaction, finalize_transaction_bytes, prepare_action,
    prepare_agent_wallet, prepare_all, prepare_create_multisig, prepare_faucet, prepare_group,
    prepare_message, prepare_multisig_approve, prepare_multisig_cancel,
    prepare_multisig_execute, prepare_multisig_propose, prepare_multisig_reject,
    prepare_create_sub_account, prepare_remove_sub_account, prepare_rename_sub_account,
    prepare_transfer, prepare_update_multisig_policy, prepare_user_settings, PreparedMessage,
};
pub use sign::Signer;
pub use types::*;

/// Re-export for convenience
pub use bs58;
pub use ed25519_dalek;
