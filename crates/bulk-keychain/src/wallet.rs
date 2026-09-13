//! Explicit wallet signature modes. Raw preparation and ordinary signing are unchanged.

use crate::{Error, PreparedMessage, Pubkey, Result, SignatureDomain, SignedTransaction, Signer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalletSignatureMode {
    Raw,
    Base58,
    Offchain,
}

impl WalletSignatureMode {
    /// Value for the Bulk signature-mode transport hint.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Base58 => "base58",
            Self::Offchain => "offchain",
        }
    }
}

impl std::str::FromStr for WalletSignatureMode {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self> {
        match value {
            "raw" => Ok(Self::Raw),
            "base58" => Ok(Self::Base58),
            "offchain" => Ok(Self::Offchain),
            _ => Err(Error::SigningFailed(
                "signature mode must be raw, base58 or offchain".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPreparedMessage {
    /// Original trusted raw preparation output. Do not edit action JSON or order IDs.
    pub prepared: PreparedMessage,
    pub signature_mode: WalletSignatureMode,
    /// Exact bytes to sign, including the envelope for Offchain mode.
    #[serde(with = "serde_bytes")]
    pub message_bytes: Vec<u8>,
    /// Display text for Offchain mode; this alone is not the signature input.
    pub clear_sign_message: Option<String>,
}

/// Select a wallet encoding for original trusted raw preparation output.
/// Offchain rejects action shapes whose canonical text has not been verified.
pub fn prepare_wallet_message(
    prepared: PreparedMessage,
    signature_mode: WalletSignatureMode,
) -> Result<WalletPreparedMessage> {
    let domain = prepared_domain(&prepared)?;
    let clear_sign_message = if signature_mode == WalletSignatureMode::Offchain {
        Some(crate::clear_sign::canonical_message(&prepared)?)
    } else {
        None
    };
    let message_bytes = match signature_mode {
        WalletSignatureMode::Raw => prepared.message_bytes.clone(),
        WalletSignatureMode::Base58 => bs58::encode(&prepared.message_bytes)
            .into_string()
            .into_bytes(),
        WalletSignatureMode::Offchain => {
            let payload = clear_sign_message
                .as_ref()
                .expect("offchain text constructed above")
                .as_bytes();
            let length = u16::try_from(payload.len())
                .map_err(|_| Error::SigningFailed("offchain message exceeds 65535 bytes".into()))?;
            let mut envelope = Vec::with_capacity(85 + payload.len());
            envelope.extend_from_slice(b"\xffsolana offchain");
            envelope.push(0);
            envelope.push(domain as u8);
            envelope.extend_from_slice(&[0; 31]);
            envelope.push(if payload.iter().all(|byte| (0x20..=0x7e).contains(byte)) {
                0
            } else {
                1
            });
            envelope.push(1);
            envelope.extend_from_slice(Pubkey::from_base58(&prepared.signer)?.as_bytes());
            envelope.extend_from_slice(&length.to_le_bytes());
            envelope.extend_from_slice(payload);
            envelope
        }
    };
    Ok(WalletPreparedMessage {
        prepared,
        signature_mode,
        message_bytes,
        clear_sign_message,
    })
}

/// Verify the selected signature bytes and finalize the original Bulk payload.
/// This retains the raw finalizer's trust requirement for action JSON and order IDs.
pub fn finalize_wallet_message(
    wallet: WalletPreparedMessage,
    signature: &str,
) -> Result<SignedTransaction> {
    let wallet = validate_wallet_message(wallet)?;
    crate::prepare::finalize_with_message(wallet.prepared, signature, Some(&wallet.message_bytes))
}

/// Sign an explicit wallet mode using the configured signer and network.
pub fn sign_wallet_message(
    signer: &Signer,
    wallet: WalletPreparedMessage,
) -> Result<SignedTransaction> {
    let wallet = validate_wallet_message(wallet)?;
    if prepared_domain(&wallet.prepared)? != signer.signature_domain()
        || Pubkey::from_base58(&wallet.prepared.signer)? != signer.pubkey()
    {
        return Err(Error::SigningFailed(
            "prepared wallet signer or network does not match configured signer".into(),
        ));
    }
    let signature = signer.sign_bytes(&wallet.message_bytes);
    crate::prepare::finalize_with_message(wallet.prepared, &signature, Some(&wallet.message_bytes))
}

fn validate_wallet_message(wallet: WalletPreparedMessage) -> Result<WalletPreparedMessage> {
    let rebuilt = prepare_wallet_message(wallet.prepared, wallet.signature_mode)?;
    if rebuilt.message_bytes != wallet.message_bytes
        || rebuilt.clear_sign_message != wallet.clear_sign_message
    {
        return Err(Error::SigningFailed(
            "wallet message differs from selected encoding of the raw preparation".into(),
        ));
    }
    Ok(rebuilt)
}

pub(crate) fn prepared_domain(prepared: &PreparedMessage) -> Result<SignatureDomain> {
    if prepared.actions.is_empty() {
        return Err(Error::EmptyOrders);
    }
    let suffix = prepared
        .message_bytes
        .len()
        .checked_sub(41)
        .and_then(|offset| prepared.message_bytes.get(offset..))
        .ok_or_else(|| {
            Error::SigningFailed("prepared message is missing its signing suffix".into())
        })?;
    if suffix[..8] != prepared.nonce.to_le_bytes()
        || suffix[8..40] != Pubkey::from_base58(&prepared.account)?.as_bytes()[..]
    {
        return Err(Error::SigningFailed(
            "prepared account or nonce differs from raw signing bytes".into(),
        ));
    }
    Pubkey::from_base58(&prepared.signer)?;
    match suffix[40] {
        1 => Ok(SignatureDomain::Mainnet),
        2 => Ok(SignatureDomain::Testnet),
        3 => Ok(SignatureDomain::Devnet),
        _ => Err(Error::SigningFailed(
            "prepared message has an invalid network domain".into(),
        )),
    }
}
