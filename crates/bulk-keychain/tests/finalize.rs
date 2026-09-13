use bulk_keychain::{
    finalize_all, finalize_transaction, finalize_transaction_bytes, prepare_message, Keypair,
    Order, SignatureDomain, Signer,
};

#[test]
fn external_signature_requires_matching_signer_and_metadata() {
    let key = Keypair::from_secret_key(&[1; 32]).unwrap();
    let signer = Signer::new(key.clone(), SignatureDomain::Devnet);
    let prepared = prepare_message(
        Order::market("BTC-USD", true, 1.0).into(),
        SignatureDomain::Devnet,
        &key.pubkey(),
        None,
        Some(42),
    )
    .unwrap();
    let signature = signer.sign_bytes(&prepared.message_bytes);
    assert!(finalize_transaction(prepared.clone(), &signature).is_ok());
    assert!(finalize_transaction_bytes(
        prepared.clone(),
        &bulk_keychain::bs58::decode(&signature).into_vec().unwrap()
    )
    .is_ok());
    assert!(finalize_all(vec![prepared.clone()], vec![&signature]).is_ok());
    assert!(finalize_all(vec![prepared.clone()], vec!["bad"]).is_err());
    assert!(finalize_transaction(prepared.clone(), "bad").is_err());
    assert!(finalize_transaction(prepared.clone(), &"1".repeat(89)).is_err());
    let mut empty_actions = prepared.clone();
    empty_actions.actions.clear();
    assert!(finalize_transaction(empty_actions, &signature).is_err());
    assert!(finalize_transaction_bytes(prepared.clone(), &[0; 63]).is_err());
    assert!(finalize_transaction_bytes(prepared.clone(), &[0; 64]).is_err());
    let other = Keypair::from_secret_key(&[2; 32])
        .unwrap()
        .pubkey()
        .to_base58();
    let mut mutated = prepared.clone();
    mutated.signer = other.clone();
    assert!(finalize_transaction(mutated, &signature).is_err());
    let mut mutated = prepared.clone();
    mutated.account = other;
    assert!(finalize_transaction(mutated, &signature).is_err());
    let mut mutated = prepared.clone();
    mutated.nonce += 1;
    assert!(finalize_transaction(mutated, &signature).is_err());
    let mut mutated = prepared.clone();
    mutated.message_bytes[0] ^= 1;
    assert!(finalize_transaction(mutated, &signature).is_err());
    let mut mutated = prepared.clone();
    mutated.message_bytes.clear();
    assert!(finalize_transaction(mutated, &signature).is_err());
    let mut mutated = prepared;
    *mutated.message_bytes.last_mut().unwrap() = 0;
    let invalid_domain_signature = signer.sign_bytes(&mutated.message_bytes);
    assert!(finalize_transaction(mutated, &invalid_domain_signature).is_err());
}
