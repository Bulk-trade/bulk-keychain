use bulk_keychain::{
    finalize_wallet_message, prepare_wallet_message, sign_wallet_message, Keypair, PreparedMessage,
    SignatureDomain, Signer, WalletSignatureMode,
};

#[test]
fn all_modes_match_real_sdk_bytes_and_signatures() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/wallet-mode-vectors.json")).unwrap();
    let signer = Signer::new(
        Keypair::from_secret_key(&[9; 32]).unwrap(),
        SignatureDomain::Devnet,
    );
    for fixture in fixtures["vectors"].as_array().unwrap() {
        let raw = PreparedMessage {
            message_bytes: hex::decode(fixture["raw_hex"].as_str().unwrap()).unwrap(),
            actions: fixture["actions"].as_array().unwrap().clone(),
            account: fixture["account"].as_str().unwrap().into(),
            signer: fixture["signer"].as_str().unwrap().into(),
            nonce: fixture["nonce"].as_str().unwrap().parse().unwrap(),
            order_id: None,
            order_ids: None,
        };
        let mode: WalletSignatureMode = fixture["mode"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .parse()
            .unwrap();
        let wallet = prepare_wallet_message(raw, mode).unwrap();
        assert_eq!(
            hex::encode(&wallet.message_bytes),
            fixture["message_hex"].as_str().unwrap()
        );
        if mode == WalletSignatureMode::Offchain {
            assert_eq!(
                wallet.clear_sign_message.as_deref(),
                fixture["canonical_text"].as_str()
            );
        } else {
            assert!(wallet.clear_sign_message.is_none());
        }
        let signature = fixture["signature"].as_str().unwrap();
        assert_eq!(
            finalize_wallet_message(wallet.clone(), signature)
                .unwrap()
                .signature,
            signature
        );
        assert_eq!(
            sign_wallet_message(&signer, wallet.clone())
                .unwrap()
                .signature,
            signature
        );
        let wrong_network = Signer::new(
            Keypair::from_secret_key(&[9; 32]).unwrap(),
            SignatureDomain::Mainnet,
        );
        assert!(sign_wallet_message(&wrong_network, wallet.clone()).is_err());
        let wrong_signer = Signer::new(
            Keypair::from_secret_key(&[8; 32]).unwrap(),
            SignatureDomain::Devnet,
        );
        assert!(sign_wallet_message(&wrong_signer, wallet.clone()).is_err());
        let mut tampered = wallet.clone();
        tampered.message_bytes[0] ^= 1;
        assert!(finalize_wallet_message(tampered, signature).is_err());
        let mut tampered = wallet.clone();
        tampered.signature_mode = if mode == WalletSignatureMode::Raw {
            WalletSignatureMode::Base58
        } else {
            WalletSignatureMode::Raw
        };
        assert!(finalize_wallet_message(tampered, signature).is_err());
        let mut tampered = wallet.clone();
        tampered.clear_sign_message = Some("different display".into());
        assert!(finalize_wallet_message(tampered, signature).is_err());
        assert!(finalize_wallet_message(wallet, "bad").is_err());
    }
}

#[test]
fn wallet_modes_preserve_agent_account_and_legacy_absent_slippage() {
    let signer = Signer::new(
        Keypair::from_secret_key(&[9; 32]).unwrap(),
        SignatureDomain::Devnet,
    );
    let account = Keypair::from_secret_key(&[8; 32]).unwrap().pubkey();
    let raw = bulk_keychain::prepare_message(
        bulk_keychain::Order::market("BTC-USD", true, 1.0).into(),
        SignatureDomain::Devnet,
        &account,
        Some(&signer.pubkey()),
        Some(42),
    )
    .unwrap();
    for mode in [
        WalletSignatureMode::Raw,
        WalletSignatureMode::Base58,
        WalletSignatureMode::Offchain,
    ] {
        let wallet = prepare_wallet_message(raw.clone(), mode).unwrap();
        let signed = sign_wallet_message(&signer, wallet.clone()).unwrap();
        assert_eq!(signed.account, account.to_base58());
        assert_eq!(signed.signer, signer.pubkey().to_base58());
        assert!(finalize_wallet_message(wallet, &signed.signature).is_ok());
    }
    let absent = prepare_wallet_message(raw.clone(), WalletSignatureMode::Offchain).unwrap();
    let mut equivalent = raw;
    equivalent.actions[0]["m"]["slippage"] = serde_json::Value::Null;
    let explicit_null = prepare_wallet_message(equivalent, WalletSignatureMode::Offchain).unwrap();
    assert_eq!(absent.message_bytes, explicit_null.message_bytes);
}

#[test]
fn offchain_rejects_unverified_shapes_and_oversized_text() {
    let signer = Keypair::from_secret_key(&[9; 32]).unwrap();
    let raw = bulk_keychain::prepare_message(
        bulk_keychain::Order::market("BTC-USD", true, 1.0).into(),
        SignatureDomain::Devnet,
        &signer.pubkey(),
        None,
        Some(42),
    )
    .unwrap();
    for action in ["of", "msp", "liq", "msu", "unknown"] {
        let mut prepared = raw.clone();
        prepared.actions = vec![serde_json::json!({action: {}})];
        assert!(prepare_wallet_message(prepared, WalletSignatureMode::Offchain).is_err());
    }
    let mut oversized = raw.clone();
    oversized.actions[0]["m"]["c"] = serde_json::Value::String("x".repeat(65536));
    assert!(prepare_wallet_message(oversized, WalletSignatureMode::Offchain).is_err());
    let mut malformed = raw;
    malformed.nonce += 1;
    for mode in [
        WalletSignatureMode::Raw,
        WalletSignatureMode::Base58,
        WalletSignatureMode::Offchain,
    ] {
        assert!(prepare_wallet_message(malformed.clone(), mode).is_err());
    }
}
