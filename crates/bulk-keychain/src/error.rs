//! Error types for bulk-keychain

use thiserror::Error;

/// All errors that can occur in bulk-keychain
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid base58 encoding
    #[error("invalid base58: {0}")]
    InvalidBase58(String),

    /// Invalid key length (expected 32 bytes for public key, 64 for secret key)
    #[error("invalid key length: expected {expected}, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },

    /// Invalid hash length (expected 32 bytes)
    #[error("invalid hash length: expected 32 bytes, got {0}")]
    InvalidHashLength(usize),

    /// Invalid signature length (expected 64 bytes)
    #[error("invalid signature length: expected 64 bytes, got {0}")]
    InvalidSignatureLength(usize),

    /// Signing failed
    #[error("signing failed: {0}")]
    SigningFailed(String),

    /// Empty orders array
    #[error("orders array cannot be empty")]
    EmptyOrders,

    /// Signature count mismatch (for batch finalization)
    #[error("signature count mismatch: expected {expected}, got {got}")]
    SignatureMismatch { expected: usize, got: usize },

    /// Invalid order parameters
    #[error("invalid order: {0}")]
    InvalidOrder(String),

    /// Serialization error
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// JSON parsing error
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Result type alias for bulk-keychain operations
pub type Result<T> = std::result::Result<T, Error>;
