# bulk-keychain

A simple high perf signing lib for BULK txns - Python bindings.

## Installation

```bash
pip install bulk-keychain
```

## Quick Start

```python
from bulk_keychain import Keypair, Signer

# Generate a new keypair
keypair = Keypair()
print(f"Public key: {keypair.pubkey}")

# Or import from base58
# keypair = Keypair.from_base58("your-secret-key...")

# Create a signer
signer = Signer(keypair)

# Sign a limit order
signed = signer.sign_order([{
    "type": "order",
    "symbol": "BTC-USD",
    "is_buy": True,
    "price": 100000.0,
    "size": 0.1,
    "order_type": {"type": "limit", "tif": "GTC"}
}])

# Submit to API
import requests
response = requests.post(
    "https://api.bulk.exchange/api/v1/order",
    json=signed
)
```

## Order Types

### Limit Order

```python
{
    "type": "order",
    "symbol": "BTC-USD",
    "is_buy": True,
    "price": 100000.0,
    "size": 0.1,
    "order_type": {"type": "limit", "tif": "GTC"}  # GTC, IOC, or ALO
}
```

### Market Order

```python
{
    "type": "order",
    "symbol": "BTC-USD",
    "is_buy": True,
    "price": 0.0,
    "size": 0.1,
    "order_type": {"type": "market", "is_market": True, "trigger_px": 0.0}
}
```

### Cancel Order

```python
{
    "type": "cancel",
    "symbol": "BTC-USD",
    "order_id": "order-id-base58"
}
```

### Cancel All Orders

```python
{
    "type": "cancel_all",
    "symbols": ["BTC-USD"]  # or [] for all symbols
}
```

## Batch Signing

For high-frequency trading, sign many transactions in parallel:

```python
# Create batches (each inner list becomes one transaction)
batches = [[order] for order in orders]

# Sign all at once - parallelizes automatically when > 10 batches
signed_txs = signer.sign_orders_batch(batches)
```

## API Reference

### Keypair

```python
# Generate new keypair
keypair = Keypair()

# Import from base58
keypair = Keypair.from_base58("secret-key-base58")

# Import from bytes (32 or 64 bytes)
keypair = Keypair.from_bytes(bytes_data)

# Properties and methods
keypair.pubkey          # Public key as base58 string
keypair.to_base58()     # Full keypair as base58 (64 bytes)
keypair.to_bytes()      # Full keypair as bytes
keypair.secret_key()    # Secret key as bytes (32 bytes)
```

### Signer

```python
# Create signer
signer = Signer(keypair)
signer = Signer.from_base58("secret-key-base58")

# With nonce management 
signer = Signer.with_nonce_manager(keypair, "timestamp")     # Use timestamp
signer = Signer.with_nonce_manager(keypair, "counter")       # Use counter
signer = Signer.with_nonce_manager(keypair, "high_frequency") # Timestamp + counter

# Sign operations
signed = signer.sign_order(orders, nonce=None)
signed = signer.sign_faucet(nonce=None)
signed = signer.sign_agent_wallet(agent_pubkey, delete=False, nonce=None)
signed = signer.sign_user_settings(max_leverage=[("BTC-USD", 5.0)], nonce=None)
signed_list = signer.sign_orders_batch(batches, base_nonce=None)
```

### Utilities

```python
from bulk_keychain import random_hash, current_timestamp, validate_pubkey, validate_hash

# Generate random hash for client order IDs
client_id = random_hash()

# Get current timestamp in milliseconds
ts = current_timestamp()

# Validate base58 strings
is_valid = validate_pubkey("pubkey-base58")
is_valid = validate_hash("hash-base58")
```

