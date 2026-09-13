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
const signer = new NativeSigner(keypair, 'devnet');

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
signer = Signer(keypair, "devnet")

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
use bulk_keychain::{Keypair, Order, SignatureDomain, Signer, TimeInForce};

// Generate or import keypair
let keypair = Keypair::generate();
// Or: Keypair::from_base58("your-secret-key...")?

// Create signer
let mut signer = Signer::new(keypair, SignatureDomain::Devnet);

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
| `signApproveBuilderCode(toPubkey, fee)` | Approve a builder-code recipient (`abc`) | `SignedTransaction` |
| `signRevokeBuilderCode(toPubkey)` | Revoke a builder-code recipient approval (`rbc`) | `SignedTransaction` |

Python method names are `sign_oracle_prices`, `sign_pyth_oracle`, `sign_whitelist_faucet`, `sign_approve_builder_code`, and `sign_revoke_builder_code`. Rust equivalents are `sign_oracle_prices`, `sign_pyth_oracle`, `sign_whitelist_faucet`, `sign_approve_builder_code`, and `sign_revoke_builder_code`.

## Importing an existing key

Pass the existing key to the constructor or use `fromBase58` / `fromBytes`
(`from_base58` / `from_bytes` in Python). Node and WASM accept a base58 string or
`Uint8Array` (including a Node Buffer); Python accepts a positional base58 string
or `bytes`. A 32-byte value is an Ed25519 seed. A 64-byte value must contain the
seed followed by its matching public key; inconsistent pairs are rejected.

```typescript
const imported = new NativeKeypair(existingBase58Key);
// Browser: new WasmKeypair(existingBase58Key)
const importedBytes = new NativeKeypair(existingKeyBytes);
```

```python
imported = Keypair(existing_base58_key)
imported_bytes = Keypair(existing_key_bytes)
```

A constructor with no argument still generates a new random keypair. Supplied
malformed keys and `null` / `None` raise an error; they never generate a replacement.

## Transaction Nonces

The Node.js and browser/WASM APIs accept transaction nonces only as unsigned
64-bit decimal strings and return prepared and signed transaction nonces in the
same string form. This preserves nanosecond values above
`Number.MAX_SAFE_INTEGER` without a JavaScript `number`/`f64` conversion:

```typescript
const nonce = '1704067200000000001';
const signed = signer.sign(order, nonce);
console.log(signed.nonce); // '1704067200000000001'
```

When omitted, the nonce defaults to the current Unix timestamp in nanoseconds.

## Builder Codes

Builder codes are optional builder-code fees for routed limit and market orders.
API JSON uses `builderCode` on orders and `abc`/`rbc` approval actions. When
`builderCode` is absent, it contributes no signing bytes.

## Pre-computed Order ID

Single-order transactions include an optional pre-computed order ID that matches BULK's network order ID generation. This lets you know the order ID **before** the node responds - useful for optimistic tracking.

Transaction signatures use canonical BULK-SDK bytes:

`signature = ed25519_sign(bincode(CommissionSignableActions) || nonce_le || account_bytes || domain_byte)`

The required domain byte is `1` for mainnet, `2` for testnet, or `3` for devnet.

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

### Without Signer/Private Key

You can compute an order ID directly from order fields + nonce + account:

```rust
use bulk_keychain::{
    compute_order_id, Order, Pubkey, TimeInForce,
};

let account = Pubkey::from_base58("your-account-pubkey")?;
let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
let order_id = compute_order_id(&order, 1704067200000, &account).to_base58();
```

```python
from bulk_keychain import compute_order_id_from_order

order_id = compute_order_id_from_order(
    {"type": "order", "symbol": "BTC-USD", "is_buy": True, "price": 100000.0, "size": 0.1},
    nonce=1704067200000,
    account="your-account-pubkey",
)

# Compact API order JSON is also supported:
order_id_compact = compute_order_id_from_order(
    {"l": {"c": "BTC-USD", "b": True, "px": 100000.0, "sz": 0.1, "r": False, "tif": "GTC"}},
    nonce=1704067200000,
    account="your-account-pubkey",
)
```

For multi-order transactions (`signGroup` / grouped batches), optional `order_ids` are available when batch order ID computation is enabled.

### Enable Batch Order IDs (Optional)

### TypeScript (Node.js)
```typescript
const signer = new NativeSigner(keypair, 'devnet');
signer.setComputeBatchOrderIds(true); // default false for max performance
const grouped = signer.signGroup([entryOrder, stopLoss, takeProfit]);
console.log(grouped.orderIds); // ["...", "...", "..."]
```

### Python
```python
signer = Signer(keypair, "devnet")
signer.set_compute_batch_order_ids(True)  # default False
grouped = signer.sign_group([entry_order, stop_loss, take_profit])
print(grouped.get("order_ids"))
```

### Rust
```rust
let mut signer = Signer::new(keypair, SignatureDomain::Devnet).with_batch_order_ids();
let grouped = signer.sign_group(bracket, None)?;
println!("Order IDs: {:?}", grouped.order_ids);
```

### Algorithm

Order IDs are derived from canonical BULK-SDK bytes for a single order action:

`order_id = SHA256(seqno_le + bincode(single_action) + account_bytes + nonce_le)` (base58)

Notes:
- `seqno` is the action index inside the transaction (auto-indexed for grouped txs, `0` for single-order txs)
- for limit/market actions, `px`/`sz` use BULK-SDK fixed-point serialization (`round(value * 1e8)` as `u64`)
- signer pubkey is not part of the order-ID hash

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
import { currentTimestamp, prepareOrder, WasmPreparedMessage } from 'bulk-keychain-wasm';

// Step 1: Prepare the message (no private key needed)
const prepared = prepareOrder(order, {
  signatureDomain: 'devnet',
  account: walletPubkey,        // The trading account
  signer: walletPubkey,         // Who signs (defaults to account)
  nonce: currentTimestamp()     // Optional decimal string; Unix ns by default
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
prepared = prepare_order(order, "devnet", account=wallet_pubkey)

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
| `prepareUpdateUserSettings(settings, options)` | Update user settings (leverage) |

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

### Local Agent Signing

When your backend/app holds the agent's private key in a `Signer`, you can sign a
prepared message for a different trading account directly — no external wallet call
needed. This covers the `account != signer` case where `signer` is the agent.

`signPrepared` (`sign_prepared` in Python) rejects the request if `prepared.signer`
does not match the signer's pubkey. For lower-level use, `signBytes` / `sign_bytes`
returns the base58 signature so you can call `finalize*` yourself.

#### TypeScript (Node.js)
```typescript
import { NativeSigner, prepareOrder } from 'bulk-keychain';

const agent = new NativeSigner(agentKeypair, 'devnet'); // holds the agent private key

// Prepare for the main trading account, signed by the agent
const prepared = prepareOrder(order, {
  signatureDomain: 'devnet',
  account: mainAccountPubkey,   // the trading account
  signer: agent.pubkey,         // the agent wallet signs
});

// Agent signs the canonical message and returns a SignedTransaction
const signed = agent.signPrepared(prepared);
```

#### TypeScript (WASM)
```typescript
import { WasmSigner, prepareOrder } from 'bulk-keychain-wasm';

const agent = new WasmSigner(agentKeypair, 'devnet');

const prepared = prepareOrder(order, {
  signatureDomain: 'devnet',
  account: mainAccountPubkey,
  signer: agent.pubkey,
});

const signed = agent.signPrepared(prepared);
```

#### Python
```python
from bulk_keychain import Signer, prepare_order

agent = Signer(agent_keypair, "devnet")

prepared = prepare_order(
    order,
    "devnet",
    account=main_account_pubkey,
    signer=agent.pubkey,
)

signed = agent.sign_prepared(prepared)
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

### Stop-Loss
```typescript
{
  type: 'stop',
  symbol: 'BTC-USD',
  isBuy: false,
  size: 0.1,
  triggerPrice: 90000,
  limitPrice: 89900,  // omit for market-style fill
}
```

### Take-Profit
```typescript
{
  type: 'takeProfit',
  symbol: 'BTC-USD',
  isBuy: false,
  size: 0.1,
  triggerPrice: 110000,
  limitPrice: 110100,  // omit for market-style fill
}
```

### Range / OCO (Stop-Loss + Take-Profit)
```typescript
{
  type: 'range',
  symbol: 'BTC-USD',
  isBuy: false,
  size: 0.1,
  pmin: 90000,    // stop-loss trigger price
  pmax: 110000,   // take-profit trigger price
  lmin: 89900,    // stop-loss limit price (omit for market-style fill)
  lmax: 110100,   // take-profit limit price (omit for market-style fill)
}
```

### Trigger Basket
Fires a set of child actions when price crosses a threshold. Nested actions may be: `stop`, `takeProfit`, `range`, `order`, `cancel`, `cancelAll`, `modify`.

```typescript
{
  type: 'trig',
  symbol: 'BTC-USD',
  isBuy: true,
  triggerPrice: 100000,
  actions: [
    { type: 'stop',       symbol: 'BTC-USD', isBuy: false, size: 0.1, triggerPrice: 95000 },
    { type: 'takeProfit', symbol: 'BTC-USD', isBuy: false, size: 0.1, triggerPrice: 110000 },
  ],
}
```

### Trailing Stop
Protective stop that follows price by a fixed distance (`trailBps`), resetting forward on favorable moves in increments of `stepBps`. Internally represented as a protective stop leg plus a rotating sentinel trigger leg.

```typescript
{
  type: 'trl',            // or 'trailingStop'
  symbol: 'BTC-USD',
  isBuy: true,            // true = protecting a long, false = protecting a short
  size: 0.25,
  trailBps: 100,          // trailing distance in basis points
  stepBps: 10,            // favorable reset step in basis points
  limitPrice: null,       // optional: omit or null for market-style trigger
}
```

### On-Fill Consequent
One-shot follow-up actions executed on the first fill of an inline trigger order.

```typescript
// Attach directly to an order; the order becomes the inline trigger.
const limitWithSL = {
  type: 'order',
  symbol: 'BTC-USD',
  isBuy: true,
  price: 95000,
  size: 0.1,
  orderType: { type: 'limit', tif: 'GTC' },
  onFill: {
    actions: [
      { type: 'stop', symbol: 'BTC-USD', isBuy: false, size: 0.1, triggerPrice: 90000 },
    ],
  },
};
const signed = prepareOrder(limitWithSL, { account, signer });
```

## Solana mainnet USDC deposit and withdrawal intent

`buildDepositInstruction(owner, amount)` and `buildWithdrawIntentInstruction(owner, amount)`
build unsigned Solana instructions for the pinned Bulk mainnet program and USDC mint.
The owner must be an on-curve wallet and its USDC associated token account must already
exist; the address is derived automatically. Amounts must be positive. JavaScript amounts
must be decimal strings in USDC base units (six decimals); Python accepts integers.
One USDC is `"1000000"`. Program, mint and token-account overrides are not accepted.

| Pinned account | Address |
| --- | --- |
| Bulk mainnet program | `BULK2CNYn3mbgfYXEXiBBFxmmDChznpjQ4oRfce8w6R4` |
| Mainnet USDC mint | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| Bulk USDC vault | `7Wpp33Dn5KKUFjaij4zKYy1XZ9kdBtHjUatAT6NcjjGt` |
| Vault token account | `HwdwwKH1tMXo7ggTKcA5cdQrpcgqSoVib2eQh3BiyEQL` |

The classic SPL Token and Associated Token programs are also pinned. Both deposit
source and withdrawal destination are the supplied wallet's USDC ATA, so these
builders cannot select a different token account. They build locally and do not
verify account existence or cluster state; use a mainnet Solana connection.

Rust exposes the same builders with `u64` amounts:

```rust
let deposit = bulk_keychain::solana::deposit(&owner_pubkey, 1_000_000)?;
let intent = bulk_keychain::solana::request_withdraw(&owner_pubkey, 1_000_000)?;
```

```typescript
import { buildDepositInstruction, buildWithdrawIntentInstruction } from 'bulk-keychain';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';

const instruction = buildDepositInstruction(wallet.publicKey.toBase58(), '1000000');
const deposit = new TransactionInstruction({
  programId: new PublicKey(instruction.programId),
  keys: instruction.accounts.map(account => ({
    pubkey: new PublicKey(account.pubkey),
    isSigner: account.isSigner,
    isWritable: account.isWritable,
  })),
  data: Buffer.from(instruction.data),
});
// Add deposit to a Solana transaction and sign/send through your wallet or client.
const withdrawalIntent = buildWithdrawIntentInstruction(wallet.publicKey.toBase58(), '1000000');
```

Browser/WASM exports the same function names and camelCase account fields; `data`
is an array of bytes (Node returns a Buffer). Python returns snake_case fields and bytes:

```python
from bulk_keychain import build_deposit_instruction, build_withdraw_intent_instruction

deposit = build_deposit_instruction(owner_pubkey, 1_000_000)
withdrawal_intent = build_withdraw_intent_instruction(owner_pubkey, 1_000_000)
# {"program_id": str, "accounts": [{"pubkey": str, "is_signer": bool,
#   "is_writable": bool}], "data": bytes}
```

Deposit transfers USDC into the vault. Withdrawal intent signals a withdrawal;
it does not itself transfer tokens. The client/wallet handles transaction assembly,
recent blockhash, signing, submission and confirmation. These are Solana instructions,
not Bulk API actions; do not pass them to Bulk `prepareOrder` or `finalizeTransaction`.

The previous Bulk-action `Withdraw` / `WithdrawLockRecover` helpers are retained for
source compatibility but return a legacy-withdrawal error. Use the Solana withdrawal
intent builder above for the current flow.

## Exporting unsigned Solana transactions

`exportDepositTransaction(owner, amount, recentBlockhash)` and
`exportWithdrawIntentTransaction(owner, amount, recentBlockhash)` return standard
base64-encoded, unsigned legacy Solana transactions. The owner is the fee payer
and sole required signer; its signature slot is zero-filled. The same mainnet
USDC restrictions as the instruction builders apply. No key or RPC connection
is needed to export these transactions.

```typescript
import { exportDepositTransaction } from 'bulk-keychain';
import { Transaction } from '@solana/web3.js';

// connection must point to Solana mainnet. Keep both values until confirmation.
const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('confirmed');
const encoded = exportDepositTransaction(wallet.publicKey.toBase58(), '1000000', blockhash);
const transaction = Transaction.from(Buffer.from(encoded, 'base64'));
const signed = await wallet.signTransaction(transaction);
const signature = await connection.sendRawTransaction(signed.serialize());
const confirmation = await connection.confirmTransaction(
  { signature, blockhash, lastValidBlockHeight }, 'confirmed',
);
if (confirmation.value.err) throw new Error(JSON.stringify(confirmation.value.err));
```

For withdrawal intent, use `exportWithdrawIntentTransaction` in the same flow.
Confirmation proves the intent landed; it does not prove withdrawal settlement.
The browser/WASM package exports the same names. Python exports
`export_deposit_transaction(owner, amount, recent_blockhash)` and
`export_withdraw_intent_transaction(...)`, taking integer base-unit amounts and
returning base64 strings for a Solana transaction library or wallet to consume.

Existing Bulk `prepareOrder` exports raw Bulk message bytes for offchain
`signMessage` and Bulk finalization. These Solana exports use wallet
`signTransaction`; do not pass their decoded bytes to Bulk `finalizeTransaction`.

## Finalizing external Bulk signatures

Finalization verifies a 64-byte Ed25519 signature against the named signer and
prepared message bytes. It also checks that the prepared account and nonce match
the signed suffix and that the network domain is recognized. Invalid signatures,
changed account/nonce metadata, empty actions and malformed action JSON raise errors.

Keep `actions` and optional order IDs from the original trusted prepare output.
Finalization does **not** reconstruct canonical signing bytes from action JSON,
so it does not prove that changed action JSON describes the signed message.
Raw `signBytes` remains a low-level arbitrary-message signing API.

Rust migration: `finalize_transaction` and `finalize_transaction_bytes` now return
`Result<SignedTransaction>`; propagate errors with `?` or handle them explicitly.
`finalize_all` returns an error if any signature fails. JavaScript/Python callers
receive exceptions on failure; browser `prepared.finalize` / `finalizeBytes` no
longer silently return null. Successful transaction output shapes are unchanged.

## Explicit Bulk wallet signature modes

The existing raw `prepareOrder` and signing APIs retain their behavior. To choose
an encoding explicitly, call `prepareWalletMessage(rawPrepared, mode)` with `raw`,
`base58`, or `offchain`. It retains the original preparation and returns the exact
`messageBytes` to sign, `signatureMode`, and optional `clearSignMessage` display text.
Offchain mode includes its complete Solana offchain envelope in `messageBytes` and
rejects action types outside the supported clear-sign subset.

```typescript
const rawPrepared = prepareOrder(order, {
  signatureDomain: 'mainnet', account: wallet.publicKey.toBase58(), nonce,
});
const prepared = prepareWalletMessage(rawPrepared, 'offchain');
// Sign exactly these bytes. Do not prepend another envelope or sign only the display text.
const { signature } = await wallet.signMessage(prepared.messageBytes);
const signed = finalizeWalletMessage(prepared, bs58.encode(signature));
await fetch(apiUrl + '/api/v1/order', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json', 'X-Bulk-Sig-Mode': prepared.signatureMode },
  // Node action JSON is a string; WASM action JSON is already an array of objects.
  body: JSON.stringify({ ...signed, actions: typeof signed.actions === 'string'
    ? JSON.parse(signed.actions) : signed.actions }),
});
```

`raw` signs canonical Bulk bytes. `base58` signs the ASCII base58 encoding of those
bytes. All returned signatures still use base58 transport encoding regardless of
mode. The `X-Bulk-Sig-Mode` header selects verification mode; it does not describe
how the signature string itself is encoded. Wallets that automatically add their
own message envelope need an integration that signs the returned bytes unchanged.

For a held key, use `signer.signWalletPrepared(prepared)`; it checks the configured
signer and network. In Python use `prepare_wallet_message(raw_prepared, mode)`,
`finalize_wallet_message(prepared, signature)` and `signer.sign_wallet_prepared(prepared)`.
Python wrapper fields are `prepared`, `signature_mode`, `message_bytes` (bytes), and
`clear_sign_message`. Node uses an object; WASM uses a class with getters and retains
its original preparation privately. Wrappers are language-specific, while the
exact signing bytes match across bindings. Keep original actions and order IDs
trusted and unchanged, as required by raw preparation/finalization.
