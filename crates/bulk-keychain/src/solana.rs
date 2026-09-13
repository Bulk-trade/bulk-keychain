//! Mainnet USDC deposit and withdrawal-intent instructions.
//!
//! Only the wallet and positive base-unit amount are caller-controlled. The
//! wallet's classic SPL USDC associated token account must already exist.
//! These descriptors need a Solana transaction and wallet signature, not Bulk
//! action signing or `finalize_transaction`. No RPC or account creation occurs.

use crate::{Error, Result};
use serde::Serialize;
use solana_pubkey::Pubkey;

pub const PROGRAM_ID: &str = "BULK2CNYn3mbgfYXEXiBBFxmmDChznpjQ4oRfce8w6R4";
pub const MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const VAULT: &str = "7Wpp33Dn5KKUFjaij4zKYy1XZ9kdBtHjUatAT6NcjjGt";
pub const VAULT_TOKEN_ACCOUNT: &str = "HwdwwKH1tMXo7ggTKcA5cdQrpcgqSoVib2eQh3BiyEQL";
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const DECIMALS: u8 = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    pub program_id: String,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

/// Transfer USDC from the wallet's associated token account into the Bulk vault.
/// `amount` is in base units: 1_000_000 is 1 USDC. Rejects zero and off-curve owners.
pub fn deposit(owner: &str, amount: u64) -> Result<Instruction> {
    instruction(owner, amount, true)
}

/// Request a withdrawal to the wallet's USDC associated token account.
/// This signals an intent; confirmation does not prove withdrawal settlement.
/// `amount` must be positive and is in USDC base units.
pub fn request_withdraw(owner: &str, amount: u64) -> Result<Instruction> {
    instruction(owner, amount, false)
}

/// Export a base64-encoded unsigned Solana deposit transaction for wallet signing.
/// The caller supplies a recent mainnet blockhash and tracks its expiry. This
/// performs no RPC and never requests, creates or uses a private key.
pub fn export_deposit_transaction(
    owner: &str,
    amount: u64,
    recent_blockhash: &str,
) -> Result<String> {
    export_transaction(deposit(owner, amount)?, recent_blockhash)
}

/// Export a base64-encoded unsigned withdrawal-intent transaction for wallet signing.
/// Confirmation of the signed intent is separate from withdrawal settlement.
pub fn export_withdraw_intent_transaction(
    owner: &str,
    amount: u64,
    recent_blockhash: &str,
) -> Result<String> {
    export_transaction(request_withdraw(owner, amount)?, recent_blockhash)
}

fn export_transaction(instruction: Instruction, recent_blockhash: &str) -> Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    // Account strings originate only from the validated pinned builders above.
    let accounts = instruction
        .accounts
        .into_iter()
        .map(|account| {
            Ok(solana_instruction::AccountMeta {
                pubkey: account
                    .pubkey
                    .parse()
                    .map_err(|_| Error::InvalidSolanaInstruction("invalid account public key"))?,
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let payer = accounts[0].pubkey;
    let message = solana_message::Message::new_with_blockhash(
        &[solana_instruction::Instruction {
            program_id: Pubkey::from_str_const(PROGRAM_ID),
            accounts,
            data: instruction.data,
        }],
        Some(&payer),
        &recent_blockhash.parse::<solana_hash::Hash>().map_err(|_| {
            Error::InvalidSolanaInstruction("recent blockhash must be a base58 32-byte hash")
        })?,
    );
    bincode::serialize(&solana_transaction::Transaction::new_unsigned(message))
        .map(|bytes| STANDARD.encode(bytes))
        .map_err(|error| Error::SerializationError(error.to_string()))
}

fn instruction(owner: &str, amount: u64, deposit: bool) -> Result<Instruction> {
    if amount == 0 {
        return Err(Error::InvalidSolanaInstruction("amount must be positive"));
    }
    let owner = owner
        .parse::<Pubkey>()
        .map_err(|_| Error::InvalidSolanaInstruction("owner must be a base58 public key"))?;
    if !owner.is_on_curve() {
        return Err(Error::InvalidSolanaInstruction(
            "owner must be an on-curve wallet",
        ));
    }
    let mut accounts = Vec::with_capacity(if deposit { 6 } else { 5 });
    accounts.extend([
        AccountMeta {
            pubkey: owner.to_string(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: Pubkey::find_program_address(
                &[
                    owner.as_ref(),
                    Pubkey::from_str_const(TOKEN_PROGRAM_ID).as_ref(),
                    Pubkey::from_str_const(MINT).as_ref(),
                ],
                &Pubkey::from_str_const(ASSOCIATED_TOKEN_PROGRAM_ID),
            )
            .0
            .to_string(),
            is_signer: false,
            is_writable: deposit,
        },
        AccountMeta {
            pubkey: VAULT.into(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: MINT.into(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: VAULT_TOKEN_ACCOUNT.into(),
            is_signer: false,
            is_writable: deposit,
        },
    ]);
    if deposit {
        accounts.push(AccountMeta {
            pubkey: TOKEN_PROGRAM_ID.into(),
            is_signer: false,
            is_writable: false,
        });
    }
    let mut data = Vec::with_capacity(9);
    data.push(if deposit { 2 } else { 4 });
    data.extend_from_slice(&amount.to_le_bytes());
    Ok(Instruction {
        program_id: PROGRAM_ID.into(),
        accounts,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_vaults_match_program_derivation() {
        assert_eq!(
            Pubkey::find_program_address(
                &[b"vault\0\0\0", Pubkey::from_str_const(MINT).as_ref()],
                &Pubkey::from_str_const(PROGRAM_ID),
            )
            .0
            .to_string(),
            VAULT
        );
        assert_eq!(
            Pubkey::find_program_address(
                &[b"vault_ata", Pubkey::from_str_const(MINT).as_ref()],
                &Pubkey::from_str_const(PROGRAM_ID),
            )
            .0
            .to_string(),
            VAULT_TOKEN_ACCOUNT
        );
    }
}
