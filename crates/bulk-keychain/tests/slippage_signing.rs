use bulk_keychain::*;
// Versioned signing fixtures covering V2 slippage and V3 builder-code layouts.
// transaction::signable_bytes_into after deserializing the public action JSON.
#[test]
fn whole_transaction_versioned_signing_matches_sdk_vectors() {
    let account = Pubkey::from_bytes([1; 32]);
    let market = OrderItem::from(Order::market("BTC-USD", true, 1.0).with_slippage(25.0));
    let plain = OrderItem::from(Order::market("BTC-USD", false, 2.0));
    let limit = OrderItem::from(Order::limit("BTC-USD", true, 100.0, 3.0, TimeInForce::Gtc));
    let mixed = Action::Order {
        orders: vec![market.clone(), plain.clone(), limit.clone()],
    };
    for (name, action) in [
        (
            "commissioned",
            Action::Order {
                orders: vec![
                    market.clone(),
                    Order::market("BTC-USD", false, 2.0)
                        .with_commission(account, 5)
                        .unwrap()
                        .into(),
                    Order::limit("BTC-USD", true, 100.0, 3.0, TimeInForce::Gtc)
                        .with_commission(account, 5)
                        .unwrap()
                        .into(),
                ],
            },
        ),
        ("mixed", mixed.clone()),
        (
            "nested",
            Action::MultisigPropose(MultisigPropose::new(
                account,
                vec![Action::MultisigPropose(MultisigPropose::new(
                    account,
                    vec![mixed.clone()],
                ))],
            )),
        ),
        (
            "trigger",
            Action::Order {
                orders: vec![OrderItem::TriggerBasket(TriggerBasket {
                    symbol: "BTC-USD".into(),
                    is_buy: true,
                    trigger_price: 100.0,
                    actions: vec![market.clone(), plain.clone(), limit.clone()],
                })],
            },
        ),
        (
            "onfill",
            Action::Order {
                orders: vec![OrderItem::OnFill(OnFill {
                    trigger: Box::new(limit.clone()),
                    actions: vec![market.clone(), plain.clone()],
                })],
            },
        ),
        (
            "legacy",
            Action::Order {
                orders: vec![plain, limit],
            },
        ),
    ] {
        let prepared =
            prepare_action(&action, SignatureDomain::Devnet, &account, None, Some(42)).unwrap();
        let expected = match name {
 "commissioned" => "ffffffffffffffff62756c6b2d616374696f6e730303000000000000000000000007000000000000004254432d5553440100e1f505000000000000000100f90295000000000000000007000000000000004254432d5553440000c2eb0b00000000000001010101010101010101010101010101010101010101010101010101010101010105000100000007000000000000004254432d5553440100e40b540200000000a3e11100000000000000000000010101010101010101010101010101010101010101010101010101010101010101052a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
 "mixed" => "ffffffffffffffff62756c6b2d616374696f6e730203000000000000000000000007000000000000004254432d5553440100e1f505000000000000000100f90295000000000000000007000000000000004254432d5553440000c2eb0b00000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e11100000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
 "nested" => "ffffffffffffffff62756c6b2d616374696f6e730201000000000000001f000000010101010101010101010101010101010101010101010101010101010101010101000000000000001f000000010101010101010101010101010101010101010101010101010101010101010103000000000000000000000007000000000000004254432d5553440100e1f505000000000000000100f90295000000000000000007000000000000004254432d5553440000c2eb0b00000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e111000000000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
 "trigger" => "ffffffffffffffff62756c6b2d616374696f6e730201000000000000000800000007000000000000004254432d5553440100e40b540200000003000000000000000000000007000000000000004254432d5553440100e1f505000000000000000100f90295000000000000000007000000000000004254432d5553440000c2eb0b00000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e11100000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
 "onfill" => "ffffffffffffffff62756c6b2d616374696f6e730201000000000000000a0000000100000007000000000000004254432d5553440100e40b540200000000a3e111000000000000000000000002000000000000000000000007000000000000004254432d5553440100e1f505000000000000000100f90295000000000000000007000000000000004254432d5553440000c2eb0b00000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
 "legacy" => "02000000000000000000000007000000000000004254432d5553440000c2eb0b0000000000000100000007000000000000004254432d5553440100e40b540200000000a3e111000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
 _ => unreachable!(),
 };
        assert_eq!(hex::encode(&prepared.message_bytes), expected, "{name}");
        let signed = Signer::new(
            Keypair::from_secret_key(&[9; 32]).unwrap(),
            SignatureDomain::Devnet,
        )
        .sign_action(&action, 42, &account)
        .unwrap();
        assert_eq!(
            signed.actions, prepared.actions,
            "{name}: signed JSON must preserve every prepared field"
        );
        assert_eq!(
            signed.signature,
            Signer::new(
                Keypair::from_secret_key(&[9; 32]).unwrap(),
                SignatureDomain::Devnet
            )
            .sign_bytes(&prepared.message_bytes),
            "{name}"
        );
    }
}

#[test]
fn limit_slippage_rejected_before_preparation_or_signing() {
    let mut signer = Signer::new(
        Keypair::from_secret_key(&[1; 32]).unwrap(),
        SignatureDomain::Devnet,
    );
    let action = Action::Order {
        orders: vec![Order::limit("BTC-USD", true, 100.0, 1.0, TimeInForce::Gtc)
            .with_slippage(25.0)
            .into()],
    };
    for action in [
        action.clone(),
        Action::MultisigPropose(MultisigPropose::new(signer.pubkey(), vec![action])),
    ] {
        assert!(matches!(
            prepare_action(
                &action,
                SignatureDomain::Devnet,
                &signer.pubkey(),
                None,
                Some(42)
            ),
            Err(Error::InvalidOrder(_))
        ));
        assert!(matches!(
            signer.sign_action_self(&action, 42),
            Err(Error::InvalidOrder(_))
        ));
    }
}

#[test]
fn conditional_orders_use_v4_with_omitted_or_explicit_slippage() {
    const V4_PREFIX: &[u8] = b"\xff\xff\xff\xff\xff\xff\xff\xffbulk-actions\x04";
    let account = Pubkey::from_bytes([4; 32]);
    let stop = OrderItem::Stop(Stop {
        symbol: "BTC-USD".into(),
        is_buy: false,
        size: 1.0,
        trigger_price: 90_000.0,
        limit_price: f64::NAN,
        iso: false,
        commission: None,
        slippage: None,
    });
    let range = OrderItem::RangeOco(RangeOco {
        symbol: "BTC-USD".into(),
        is_buy: true,
        size: 1.0,
        collar_min: 90_000.0,
        collar_max: 110_000.0,
        limit_min: f64::NAN,
        limit_max: f64::NAN,
        iso: false,
        commission: None,
        sl_slippage: Some(25.0),
        tp_slippage: Some(50.0),
    });
    let trailing = OrderItem::TrailingStop(TrailingStop {
        symbol: "BTC-USD".into(),
        is_buy: true,
        size: 1.0,
        trail_bps: 100,
        step_bps: 10,
        limit_price: None,
        iso: false,
        commission: None,
        slippage: None,
    });

    for orders in [
        vec![stop],
        vec![range],
        vec![OrderItem::TriggerBasket(TriggerBasket {
            symbol: "BTC-USD".into(),
            is_buy: true,
            trigger_price: 100_000.0,
            actions: vec![trailing],
        })],
    ] {
        let prepared = prepare_action(
            &Action::Order { orders },
            SignatureDomain::Devnet,
            &account,
            None,
            Some(42),
        )
        .unwrap();
        assert!(prepared.message_bytes.starts_with(V4_PREFIX));
    }
}

#[test]
fn on_fill_range_with_market_slippage_matches_sdk_v4_vector() {
    const SDK_SIGNABLE_HEX: &str = "ffffffffffffffff62756c6b2d616374696f6e730401000000000000000a000000000000000700000000000000534f4c2d555344017fdd5c84000000000000000100e269b60c0000000100000000000000070000000700000000000000534f4c2d555344017fdd5c8400000000001374ad0200000000dd0ee902000000000000000000009f41f3f546db18b5aec8bd97efc4b9d71ea4b7d83b270495a4bc9b77f8d08b121169783fe0167c01";
    let account = Pubkey::from_base58("DEDNehoFBw8Ac33gZPdNrc7EZREFcjuJ93Gj2QR6Q9hd").unwrap();
    let action = Action::Order {
        orders: vec![OrderItem::OnFill(OnFill {
            trigger: Box::new(
                Order::market("SOL-USD", true, 22.206_785_27)
                    .with_slippage(546.0)
                    .into(),
            ),
            actions: vec![OrderItem::RangeOco(RangeOco {
                symbol: "SOL-USD".into(),
                is_buy: true,
                size: 22.206_785_27,
                collar_min: 115.0,
                collar_max: 125.0,
                limit_min: f64::NAN,
                limit_max: f64::NAN,
                iso: false,
                commission: None,
                sl_slippage: None,
                tp_slippage: None,
            })],
        })],
    };

    let prepared = prepare_action(
        &action,
        SignatureDomain::Mainnet,
        &account,
        Some(&Pubkey::from_base58("H2b1D7TCCmhA1S5KUEVq6d8VWcU4CL8i3NuGxHmKxuCk").unwrap()),
        Some(1_791_103_298_972_000_000),
    )
    .unwrap();

    assert!(prepared
        .message_bytes
        .starts_with(b"\xff\xff\xff\xff\xff\xff\xff\xffbulk-actions\x04"));
    assert_eq!(hex::encode(prepared.message_bytes), SDK_SIGNABLE_HEX);
}
