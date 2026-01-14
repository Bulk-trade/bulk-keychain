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
    ...JSON.parse(signed.action),
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
requests.post('https://api.bulk.exchange/api/v1/order', json=signed)
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
