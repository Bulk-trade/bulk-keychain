"""
Basic Python example for bulk-keychain

Run: python examples/python/basic.py
"""

import time
from bulk_keychain import Keypair, Signer, random_hash, current_timestamp

def main():
    print("=== BULK Keychain Example ===\n")

    # 1. Generate a new keypair
    keypair = Keypair()
    print("Generated keypair:")
    print(f"  Public key: {keypair.pubkey}")
    print(f"  Secret (base58): {keypair.to_base58()[:20]}...")
    print()

    # 2. Create a signer
    signer = Signer(keypair)
    print(f"Signer pubkey: {signer.pubkey}")
    print()

    # 3. Sign a single limit order (using new simple API)
    print("--- Single Order (sign) ---")
    limit_order = {
        "type": "order",
        "symbol": "BTC-USD",
        "is_buy": True,
        "price": 100000.0,
        "size": 0.1,
        "order_type": {"type": "limit", "tif": "GTC"}
    }
    signed_limit = signer.sign(limit_order)
    print(f"Signature: {signed_limit['signature'][:40]}...")
    print()

    # 4. Sign a market order
    print("--- Market Order ---")
    market_order = {
        "type": "order",
        "symbol": "ETH-USD",
        "is_buy": False,
        "price": 0.0,
        "size": 1.0,
        "order_type": {"type": "market", "is_market": True, "trigger_px": 0.0}
    }
    signed_market = signer.sign(market_order)
    print(f"Action type: {signed_market['action']['type']}")
    print()

    # 5. Sign multiple orders atomically (sign_group)
    print("--- Atomic Bracket Order (sign_group) ---")
    bracket = [
        {
            "type": "order",
            "symbol": "BTC-USD",
            "is_buy": True,
            "price": 100000.0,
            "size": 0.1,
            "order_type": {"type": "limit", "tif": "GTC"}
        },
        {
            "type": "order",
            "symbol": "BTC-USD",
            "is_buy": False,
            "price": 99000.0,
            "size": 0.1,
            "order_type": {"type": "limit", "tif": "GTC"}
        },
        {
            "type": "order",
            "symbol": "BTC-USD",
            "is_buy": False,
            "price": 110000.0,
            "size": 0.1,
            "order_type": {"type": "limit", "tif": "GTC"}
        }
    ]
    signed_bracket = signer.sign_group(bracket)
    print(f"Bracket order: {len(signed_bracket['action']['orders'])} orders in 1 tx")
    print()

    # 6. Cancel order
    print("--- Cancel Order ---")
    cancel_order = {
        "type": "cancel",
        "symbol": "BTC-USD",
        "order_id": random_hash()
    }
    signed_cancel = signer.sign(cancel_order)
    print(f"Cancel signature: {signed_cancel['signature'][:40]}...")
    print()

    # 7. Cancel all orders
    print("--- Cancel All ---")
    cancel_all = {
        "type": "cancel_all",
        "symbols": ["BTC-USD", "ETH-USD"]
    }
    signed_cancel_all = signer.sign(cancel_all)
    print(f"CancelAll signature: {signed_cancel_all['signature'][:40]}...")
    print()

    # 8. Batch signing - each order gets its own tx (sign_all) - HFT optimized
    print("--- Batch Signing (sign_all - 100 orders) ---")
    orders = [
        {
            "type": "order",
            "symbol": "BTC-USD",
            "is_buy": i % 2 == 0,
            "price": 100000.0 + i * 10,
            "size": 0.01,
            "order_type": {"type": "limit", "tif": "GTC"}
        }
        for i in range(100)
    ]

    start_time = time.perf_counter()
    signed_batch = signer.sign_all(orders)
    elapsed = (time.perf_counter() - start_time) * 1000

    print(f"Signed {len(signed_batch)} transactions in {elapsed:.2f}ms")
    print(f"Throughput: {len(signed_batch) / elapsed * 1000:.0f} tx/sec")
    print()

    # 9. Sign faucet request
    print("--- Faucet Request ---")
    signed_faucet = signer.sign_faucet()
    print(f"Faucet action type: {signed_faucet['action']['type']}")
    print()

    # 10. Sign user settings
    print("--- User Settings (Leverage) ---")
    signed_settings = signer.sign_user_settings([
        ("BTC-USD", 5.0),
        ("ETH-USD", 3.0)
    ])
    print(f"Settings action type: {signed_settings['action']['type']}")

    print("\n=== Done ===")


if __name__ == "__main__":
    main()
