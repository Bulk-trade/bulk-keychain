//! Basic Rust example for bulk-keychain
//!
//! Run: cargo run --example basic

use bulk_keychain::{
    Cancel, CancelAll, Hash, Keypair, Order, OrderItem, Signer, TimeInForce, UserSettings,
};
use std::time::Instant;

fn main() -> bulk_keychain::Result<()> {
    println!("=== BULK Keychain Example ===\n");

    // 1. Generate a new keypair
    let keypair = Keypair::generate();
    println!("Generated keypair:");
    println!("  Public key: {}", keypair.pubkey());
    let b58 = keypair.to_base58();
    println!("  Secret (base58): {}...", &b58[..20]);
    println!();

    // 2. Create a signer
    let mut signer = Signer::new(keypair);
    println!("Signer pubkey: {}", signer.pubkey());
    println!();

    // 3. Sign a single limit order (using new simple API)
    println!("--- Single Order (sign) ---");
    let limit_order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
    let signed_limit = signer.sign(limit_order.into(), None)?;
    println!("Signature: {}...", &signed_limit.signature[..40]);
    println!();

    // 4. Sign a market order
    println!("--- Market Order ---");
    let market_order = Order::market("ETH-USD", false, 1.0);
    let signed_market = signer.sign(market_order.into(), None)?;
    println!("Action type: {}", signed_market.action["type"]);
    println!();

    // 5. Sign multiple orders atomically (sign_group)
    println!("--- Atomic Bracket Order (sign_group) ---");
    let bracket: Vec<OrderItem> = vec![
        Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc).into(),  // Entry
        Order::limit("BTC-USD", false, 99000.0, 0.1, TimeInForce::Gtc).into(),  // Stop loss
        Order::limit("BTC-USD", false, 110000.0, 0.1, TimeInForce::Gtc).into(), // Take profit
    ];
    let signed_bracket = signer.sign_group(bracket, None)?;
    println!("Bracket order: {} orders in 1 tx", signed_bracket.action["orders"].as_array().unwrap().len());
    println!();

    // 6. Cancel order
    println!("--- Cancel Order ---");
    let cancel = Cancel::new("BTC-USD", Hash::random());
    let signed_cancel = signer.sign(cancel.into(), None)?;
    println!("Cancel signature: {}...", &signed_cancel.signature[..40]);
    println!();

    // 7. Cancel all orders
    println!("--- Cancel All ---");
    let cancel_all = CancelAll::for_symbols(vec!["BTC-USD".into(), "ETH-USD".into()]);
    let signed_cancel_all = signer.sign(cancel_all.into(), None)?;
    println!("CancelAll signature: {}...", &signed_cancel_all.signature[..40]);
    println!();

    // 8. Batch signing - each order gets its own tx (sign_all) 
    println!("--- Batch Signing (sign_all - 100 orders) ---");
    let orders: Vec<OrderItem> = (0..100)
        .map(|i| {
            Order::limit(
                "BTC-USD",
                i % 2 == 0,
                100000.0 + i as f64 * 10.0,
                0.01,
                TimeInForce::Gtc,
            )
            .into()
        })
        .collect();

    let start = Instant::now();
    let signed_batch = signer.sign_all(orders, None)?;
    let elapsed = start.elapsed();

    println!(
        "Signed {} transactions in {:.2}ms",
        signed_batch.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "Throughput: {:.0} tx/sec",
        signed_batch.len() as f64 / elapsed.as_secs_f64()
    );
    println!();

    // 9. Sign faucet request
    println!("--- Faucet Request ---");
    let signed_faucet = signer.sign_faucet(None)?;
    println!("Faucet action type: {}", signed_faucet.action["type"]);
    println!();

    // 10. Sign user settings
    println!("--- User Settings (Leverage) ---");
    let settings = UserSettings::new(vec![("BTC-USD".into(), 5.0), ("ETH-USD".into(), 3.0)]);
    let signed_settings = signer.sign_user_settings(settings, None)?;
    println!("Settings action type: {}", signed_settings.action["type"]);

    println!("\n=== Done ===");
    Ok(())
}
