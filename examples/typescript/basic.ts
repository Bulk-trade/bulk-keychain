/**
 * Basic TypeScript example for bulk-keychain
 *
 * Run: npx ts-node examples/typescript/basic.ts
 */

import { NativeKeypair, NativeSigner, randomHash } from "bulk-keychain";

async function main() {
  console.log("=== BULK Keychain Example ===\n");

  // 1. Generate a new keypair
  const keypair = new NativeKeypair();
  console.log("Generated keypair:");
  console.log(`  Public key: ${keypair.pubkey}`);
  console.log(`  Secret (base58): ${keypair.toBase58().slice(0, 20)}...`);
  console.log();

  // 2. Create a signer
  const signer = new NativeSigner(keypair);
  console.log(`Signer pubkey: ${signer.pubkey}`);
  console.log();

  // 3. Sign a single limit order (using new simple API)
  console.log("--- Single Order (sign) ---");
  const signedLimit = signer.sign({
    type: "order",
    symbol: "BTC-USD",
    isBuy: true,
    price: 100000,
    size: 0.1,
    orderType: { type: "limit", tif: "GTC" },
  });
  console.log(`Signature: ${signedLimit.signature.slice(0, 40)}...`);
  console.log();

  // 4. Sign a market order
  console.log("--- Market Order ---");
  const signedMarket = signer.sign({
    type: "order",
    symbol: "ETH-USD",
    isBuy: false,
    price: 0,
    size: 1.0,
    orderType: { type: "market", isMarket: true, triggerPx: 0 },
  });
  console.log(`Action: ${signedMarket.action}`);
  console.log();

  // 5. Sign multiple orders atomically (signGroup)
  console.log("--- Atomic Bracket Order (signGroup) ---");
  const signedBracket = signer.signGroup([
    {
      type: "order",
      symbol: "BTC-USD",
      isBuy: true,
      price: 100000,
      size: 0.1,
      orderType: { type: "limit", tif: "GTC" },
    },
    {
      type: "order",
      symbol: "BTC-USD",
      isBuy: false,
      price: 99000,
      size: 0.1,
      orderType: { type: "limit", tif: "GTC" }, // Stop loss
    },
    {
      type: "order",
      symbol: "BTC-USD",
      isBuy: false,
      price: 110000,
      size: 0.1,
      orderType: { type: "limit", tif: "GTC" }, // Take profit
    },
  ]);
  const bracketAction = JSON.parse(signedBracket.action);
  console.log(`Bracket order: ${bracketAction.orders.length} orders in 1 tx`);
  console.log();

  // 6. Cancel order
  console.log("--- Cancel Order ---");
  const signedCancel = signer.sign({
    type: "cancel",
    symbol: "BTC-USD",
    orderId: randomHash(),
  });
  console.log(`Cancel signature: ${signedCancel.signature.slice(0, 40)}...`);
  console.log();

  // 7. Cancel all orders
  console.log("--- Cancel All ---");
  const signedCancelAll = signer.sign({
    type: "cancelAll",
    symbols: ["BTC-USD", "ETH-USD"],
  });
  console.log(
    `CancelAll signature: ${signedCancelAll.signature.slice(0, 40)}...`
  );
  console.log();

  // 8. Batch signing - each order gets its own tx (signAll) - HFT optimized
  console.log("--- Batch Signing (signAll - 100 orders) ---");
  const orders = Array.from({ length: 100 }, (_, i) => ({
    type: "order" as const,
    symbol: "BTC-USD",
    isBuy: i % 2 === 0,
    price: 100000 + i * 10,
    size: 0.01,
    orderType: { type: "limit" as const, tif: "GTC" as const },
  }));

  const startTime = performance.now();
  const signedBatch = signer.signAll(orders);
  const elapsed = performance.now() - startTime;

  console.log(
    `Signed ${signedBatch.length} transactions in ${elapsed.toFixed(2)}ms`
  );
  console.log(
    `Throughput: ${((signedBatch.length / elapsed) * 1000).toFixed(0)} tx/sec`
  );
  console.log();

  // 9. Sign faucet request
  console.log("--- Faucet Request ---");
  const signedFaucet = signer.signFaucet();
  const faucetAction = JSON.parse(signedFaucet.action);
  console.log(`Faucet action type: ${faucetAction.type}`);
  console.log();

  // 10. Sign user settings
  console.log("--- User Settings (Leverage) ---");
  const signedSettings = signer.signUserSettings([
    { symbol: "BTC-USD", leverage: 5.0 },
    { symbol: "ETH-USD", leverage: 3.0 },
  ]);
  const settingsAction = JSON.parse(signedSettings.action);
  console.log(`Settings action type: ${settingsAction.type}`);

  console.log("\n=== Done ===");
}

main().catch(console.error);
