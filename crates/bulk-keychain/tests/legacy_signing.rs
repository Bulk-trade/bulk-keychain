use bulk_keychain::*;

// Covers V12 transactions with nested multisig/on-fill actions and builder codes.
#[test]
fn v12_multisig_and_builder_signing_match_sdk_vectors() {
    let account = Pubkey::from_bytes([1; 32]);
    let plain = OrderItem::from(Order::market("BTC-USD", false, 2.0));
    let limit = OrderItem::from(Order::limit("BTC-USD", true, 100.0, 3.0, TimeInForce::Gtc));
    for (name, action) in [
        (
            "legacy_multisig_onfill_trigger",
            Action::MultisigPropose(MultisigPropose::new(
                account,
                vec![Action::Order {
                    orders: vec![OrderItem::OnFill(OnFill {
                        trigger: Box::new(plain.clone()),
                        actions: vec![OrderItem::TriggerBasket(TriggerBasket {
                            symbol: "BTC-USD".into(),
                            is_buy: true,
                            trigger_price: 100.0,
                            actions: vec![plain.clone(), limit.clone()],
                        })],
                    })],
                }],
            )),
        ),
        (
            "legacy",
            Action::Order {
                orders: vec![plain.clone(), limit.clone()],
            },
        ),
        (
            "builder_v3_market_commission",
            Action::Order {
                orders: vec![Order::market("BTC-USD", true, 1.0)
                    .with_commission(account, 5)
                    .unwrap()
                    .into()],
            },
        ),
        (
            "builder_v3_limit_commission",
            Action::Order {
                orders: vec![Order::limit("BTC-USD", true, 100.0, 3.0, TimeInForce::Gtc)
                    .with_commission(account, 5)
                    .unwrap()
                    .into()],
            },
        ),
        (
            "legacy_multisig_market",
            Action::MultisigPropose(MultisigPropose::new(
                account,
                vec![Action::Order {
                    orders: vec![plain.clone()],
                }],
            )),
        ),
        (
            "legacy_multisig_limit",
            Action::MultisigPropose(MultisigPropose::new(
                account,
                vec![Action::Order {
                    orders: vec![limit.clone()],
                }],
            )),
        ),
        (
            "legacy_multisig_nested",
            Action::MultisigPropose(MultisigPropose::new(
                account,
                vec![Action::MultisigPropose(MultisigPropose::new(
                    account,
                    vec![Action::Order {
                        orders: vec![plain.clone(), limit.clone()],
                    }],
                ))],
            )),
        ),
    ] {
        let expected = match name {
"legacy_multisig_onfill_trigger" => "0c00000001000000000000001f000000010101010101010101010101010101010101010101010101010101010101010101000000000000000a0000000000000007000000000000004254432d5553440000c2eb0b000000000000000001000000000000000800000007000000000000004254432d5553440100e40b540200000002000000000000000000000007000000000000004254432d5553440000c2eb0b00000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e1110000000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
"legacy" => "0c00000002000000000000000000000007000000000000004254432d5553440000c2eb0b00000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e11100000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
"builder_v3_market_commission" => "0c00000001000000000000000000000007000000000000004254432d5553440100e1f50500000000000001010101010101010101010101010101010101010101010101010101010101010105002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
"builder_v3_limit_commission" => "0c00000001000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e11100000000000000000000010101010101010101010101010101010101010101010101010101010101010101052a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
"legacy_multisig_market" => "0c00000001000000000000001f000000010101010101010101010101010101010101010101010101010101010101010101000000000000000000000007000000000000004254432d5553440000c2eb0b0000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
"legacy_multisig_limit" => "0c00000001000000000000001f000000010101010101010101010101010101010101010101010101010101010101010101000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e1110000000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
"legacy_multisig_nested" => "0c00000001000000000000001f000000010101010101010101010101010101010101010101010101010101010101010101000000000000001f000000010101010101010101010101010101010101010101010101010101010101010102000000000000000000000007000000000000004254432d5553440000c2eb0b00000000000000000100000007000000000000004254432d5553440100e40b540200000000a3e111000000000000000000000000002a00000000000000010101010101010101010101010101010101010101010101010101010101010103",
_ => unreachable!(),
 };
        let actual = hex::encode(
            prepare_action(&action, SignatureDomain::Devnet, &account, None, Some(42))
                .unwrap()
                .message_bytes,
        );
        assert_eq!(actual, expected, "{name}");
    }
}
