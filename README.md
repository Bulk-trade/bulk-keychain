# bulk-keychain

A simple high perf signing lib for BULK txns.

One Rust core, bindings for TypeScript, Python, and direct Rust usage.

## Packages

| Package | Description | Install |
|---------|-------------|---------|
| `bulk-keychain` | TypeScript/JavaScript (Node.js) | `npm install bulk-keychain` |
| `bulk-keychain-wasm` | TypeScript/JavaScript (Browser) | `npm install bulk-keychain-wasm` |
| `bulk-keychain` | Python | `pip install bulk-keychain` |
| `bulk-keychain` | Rust crate | `cargo add bulk-keychain` |



## TypeScript (Node.js)

```typescript
import { NativeKeypair, NativeSigner, randomHash } from 'bulk-keychain';

// Generate or import keypair
const keypair = new NativeKeypair();
// Or: NativeKeypair.fromBase58('your-secret-key...')

// Create signer
const signer = new NativeSigner(keypair);

// Sign a single order
const signed = signer.sign({
  type: 'order',
  symbol: 'BTC-USD',
  isBuy: true,
  price: 100000,
  size: 0.1,
  orderType: { type: 'limit', tif: 'GTC' }
});

// Submit to API
await fetch('https://api.bulk.exchange/api/v1/order', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    actions: JSON.parse(signed.actions),
    nonce: signed.nonce,
    account: signed.account,
    signer: signed.signer,
    signature: signed.signature
  })
});
```

## Python

```python
from bulk_keychain import Keypair, Signer

# Generate or import keypair
keypair = Keypair()
# Or: Keypair.from_base58('your-secret-key...')

# Create signer
signer = Signer(keypair)

# Sign a single order
signed = signer.sign({
    "type": "order",
    "symbol": "BTC-USD",
    "is_buy": True,
    "price": 100000.0,
    "size": 0.1,
    "order_type": {"type": "limit", "tif": "GTC"}
})

# Submit to API
import requests
requests.post(
    'https://api.bulk.exchange/api/v1/order',
    json={
        "actions": signed["actions"],
        "nonce": signed["nonce"],
        "account": signed["account"],
        "signer": signed["signer"],
        "signature": signed["signature"],
    },
)
```

## Rust

```rust
use bulk_keychain::{Keypair, Signer, Order, TimeInForce};

// Generate or import keypair
let keypair = Keypair::generate();
// Or: Keypair::from_base58("your-secret-key...")?

// Create signer
let mut signer = Signer::new(keypair);

// Sign a single order
let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
let signed = signer.sign(order.into(), None)?;

// Serialize to JSON
let json = signed.to_json()?;
```

## API Overview

| Method | Description | Returns |
|--------|-------------|---------|
| `sign(order)` | Sign a single order/cancel | `SignedTransaction` |
| `signAll([orders])` | Sign multiple orders (each gets own tx, parallel) | `SignedTransaction[]` |
| `signGroup([orders])` | Sign multiple orders atomically (one tx) | `SignedTransaction` |
| `signOraclePrices([{ timestamp, asset, price }])` | Sign oracle `px` updates | `SignedTransaction` |
| `signPythOracle([{ timestamp, feedIndex, price, exponent }])` | Sign Pyth oracle `o` batch | `SignedTransaction` |
| `signWhitelistFaucet(targetPubkey, whitelist)` | Sign whitelist faucet admin action | `SignedTransaction` |

Python method names are `sign_oracle_prices`, `sign_pyth_oracle`, and `sign_whitelist_faucet`. Rust equivalents are `sign_oracle_prices`, `sign_pyth_oracle`, and `sign_whitelist_faucet`.

## Pre-computed Order ID

Single-order transactions include an optional pre-computed order ID that matches BULK's network order ID generation. This lets you know the order ID **before** the node responds - useful for optimistic tracking.

### TypeScript
```typescript
const signed = signer.sign(order);
console.log(`Order ID: ${signed.orderId}`);  // Optional
```

### Python
```python
signed = signer.sign(order)
print(f"Order ID: {signed.get('order_id')}")  # Optional
```

### Rust
```rust
let signed = signer.sign(order.into(), None)?;
println!("Order ID: {:?}", signed.order_id);
```

For multi-order transactions (`signGroup` / grouped batches), optional `order_ids` are available when batch order ID computation is enabled.

### Enable Batch Order IDs (Optional)

### TypeScript (Node.js)
```typescript
const signer = new NativeSigner(keypair);
signer.setComputeBatchOrderIds(true); // default false for max performance
const grouped = signer.signGroup([entryOrder, stopLoss, takeProfit]);
console.log(grouped.orderIds); // ["...", "...", "..."]
```

### Python
```python
signer = Signer(keypair)
signer.set_compute_batch_order_ids(True)  # default False
grouped = signer.sign_group([entry_order, stop_loss, take_profit])
print(grouped.get("order_ids"))
```

### Rust
```rust
let mut signer = Signer::new(keypair).with_batch_order_ids();
let grouped = signer.sign_group(bracket, None)?;
println!("Order IDs: {:?}", grouped.order_ids);
```

### Algorithm

Order IDs are derived from canonical **wincode** bytes for a single order action:

`[action_count=1] + [order_action] + [nonce] + [account] + [signer]`

Then hashed as:

`order_id = SHA256(wincode_bytes)` (base58 encoded)

Encoding matches the API signing guide:
- enum discriminants: `u32` LE
- vector/string lengths: `u64` LE
- floats: raw IEEE-754 `f64` LE (no fixed-point conversion)

## Batch Signing

For high-frequency trading, sign many independent orders in parallel:

### TypeScript
```typescript
// Each order becomes its own transaction (parallel signing)
const orders = [order1, order2, order3];
const signedTxs = signer.signAll(orders);  // Returns SignedTransaction[]
```

### Python
```python
# Each order becomes its own transaction (parallel signing)
orders = [order1, order2, order3]
signed_txs = signer.sign_all(orders)  # Returns list of dicts
```

### Rust
```rust
// Each order becomes its own transaction (parallel signing)
let orders = vec![order1.into(), order2.into(), order3.into()];
let signed_txs = signer.sign_all(orders, None)?;  // Returns Vec<SignedTransaction>
```

## Atomic Multi-Order (Bracket Orders)

For bracket orders (entry + stop loss + take profit) that must succeed or fail together:

### TypeScript
```typescript
// All orders in ONE transaction
const bracket = [entryOrder, stopLoss, takeProfit];
const signed = signer.signGroup(bracket);  // Returns single SignedTransaction
```

### Python
```python
# All orders in ONE transaction
bracket = [entry_order, stop_loss, take_profit]
signed = signer.sign_group(bracket)  # Returns single dict
```

### Rust
```rust
// All orders in ONE transaction
let bracket = vec![entry.into(), stop_loss.into(), take_profit.into()];
let signed = signer.sign_group(bracket, None)?;  // Returns SignedTransaction
```

## External Wallet Support (Phantom, Privy, etc.)

For browser apps using external wallets where you don't have access to the private key, use the **prepare/finalize** flow:

### TypeScript (WASM)
```typescript
import { prepareOrder, WasmPreparedMessage } from 'bulk-keychain-wasm';

// Step 1: Prepare the message (no private key needed)
const prepared = prepareOrder(order, {
  account: walletPubkey,        // The trading account
  signer: walletPubkey,         // Who signs (defaults to account)
  nonce: Date.now()             // Optional, auto-generated if omitted
});

// Step 2: Get signature from external wallet
// prepared.messageBytes is Uint8Array - pass to wallet.signMessage()
const { signature } = await wallet.signMessage(prepared.messageBytes);

// Step 3: Finalize into SignedTransaction
const signed = prepared.finalize(bs58.encode(signature));

// Alternative format options:
prepared.messageBase58;  // Base58 encoded message
prepared.messageBase64;  // Base64 encoded message  
prepared.messageHex;     // Hex encoded message
prepared.orderId;        // Optional pre-computed order ID
```

### Python
```python
from bulk_keychain import prepare_order, finalize_transaction

# Step 1: Prepare
prepared = prepare_order(order, account=wallet_pubkey)

# Step 2: Sign with external wallet
signature = wallet.sign_message(prepared["message_bytes"])

# Step 3: Finalize
signed = finalize_transaction(prepared, signature)
```

### Prepare Functions

| Function | Description |
|----------|-------------|
| `prepareOrder(order, options)` | Single order |
| `prepareAll(orders, options)` | Multiple orders (parallel, each gets own tx) |
| `prepareGroup(orders, options)` | Atomic multi-order (one tx) |
| `prepareAgentWallet(agent, delete, options)` | Agent wallet authorization |
| `prepareFaucet(options)` | Testnet faucet request |

### Agent Wallet with External Signing

When the main account uses an external wallet but trades via an agent:

```typescript
// Main wallet (Phantom) authorizes agent wallet (Privy)
const prepared = prepareAgentWallet(agentPubkey, false, {
  account: mainWalletPubkey,  // Phantom
  signer: mainWalletPubkey    // Phantom signs
});

const { signature } = await phantom.signMessage(prepared.messageBytes);
const signed = prepared.finalize(bs58.encode(signature));
```

## Order Types

### Limit Order
```typescript
{
  type: 'order',
  symbol: 'BTC-USD',
  isBuy: true,
  price: 100000,
  size: 0.1,
  orderType: { type: 'limit', tif: 'GTC' }  // GTC, IOC, or ALO
}
```

### Market Order
```typescript
{
  type: 'order',
  symbol: 'BTC-USD',
  isBuy: true,
  price: 0,
  size: 0.1,
  orderType: { type: 'market', isMarket: true, triggerPx: 0 }
}
```

### Cancel Order
```typescript
{
  type: 'cancel',
  symbol: 'BTC-USD',
  orderId: 'order-id-base58'
}
```

### Cancel All
```typescript
{
  type: 'cancelAll',
  symbols: ['BTC-USD']  // or [] for all symbols
}
```
