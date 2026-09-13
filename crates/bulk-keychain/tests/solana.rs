use bulk_keychain::solana::{
    deposit, request_withdraw, MINT, PROGRAM_ID, TOKEN_PROGRAM_ID, VAULT, VAULT_TOKEN_ACCOUNT,
};

#[test]
fn pinned_deposit_and_intent_layouts() {
    // Use a deterministic signing key, not a live wallet.
    let owner = bulk_keychain::Keypair::from_secret_key(&[1; 32])
        .unwrap()
        .pubkey()
        .to_base58();
    let deposit = deposit(&owner, 1_000_000).unwrap();
    let withdraw = request_withdraw(&owner, 1_000_000).unwrap();
    assert_eq!(deposit.program_id, PROGRAM_ID);
    assert_eq!(withdraw.program_id, PROGRAM_ID);
    assert_eq!(
        deposit.data,
        [vec![2], 1_000_000u64.to_le_bytes().to_vec()].concat()
    );
    assert_eq!(
        withdraw.data,
        [vec![4], 1_000_000u64.to_le_bytes().to_vec()].concat()
    );
    assert_eq!(deposit.accounts.len(), 6);
    assert_eq!(withdraw.accounts.len(), 5);
    assert_eq!(deposit.accounts[0].pubkey, owner);
    assert!(deposit.accounts[0].is_signer && deposit.accounts[0].is_writable);
    assert_eq!(deposit.accounts[2].pubkey, VAULT);
    assert_eq!(deposit.accounts[3].pubkey, MINT);
    assert_eq!(deposit.accounts[4].pubkey, VAULT_TOKEN_ACCOUNT);
    assert_eq!(deposit.accounts[5].pubkey, TOKEN_PROGRAM_ID);
    assert_eq!(deposit.accounts[1].pubkey, withdraw.accounts[1].pubkey);
    assert!(deposit.accounts[1].is_writable && deposit.accounts[4].is_writable);
    assert!(deposit.accounts[1..].iter().all(|a| !a.is_signer));
    assert!(withdraw.accounts[1..]
        .iter()
        .all(|a| !a.is_signer && !a.is_writable));
    assert!(deposit.accounts[2..4].iter().all(|a| !a.is_writable));
    assert!(!deposit.accounts[5].is_writable);
}

#[test]
fn rejects_invalid_inputs_and_preserves_u64_amount() {
    let owner = bulk_keychain::Keypair::from_secret_key(&[1; 32])
        .unwrap()
        .pubkey()
        .to_base58();
    assert!(deposit(&owner, 0).is_err());
    assert!(request_withdraw(&owner, 0).is_err());
    assert!(deposit("invalid", 1).is_err());
    assert!(deposit(VAULT, 1).is_err());
    assert_eq!(
        &request_withdraw(&owner, u64::MAX).unwrap().data[1..],
        &u64::MAX.to_le_bytes()
    );
    assert_ne!(
        deposit(&owner, 1).unwrap().accounts[1].pubkey,
        deposit(
            &bulk_keychain::Keypair::from_secret_key(&[2; 32])
                .unwrap()
                .pubkey()
                .to_base58(),
            1
        )
        .unwrap()
        .accounts[1]
            .pubkey
    );
}
