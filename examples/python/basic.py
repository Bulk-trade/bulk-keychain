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
    signer.set_compute_batch_order_ids(True)
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
    market_tag = next(iter(signed_market["actions"][0].keys()))
    print(f"Action tag: {market_tag}")
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
    print(f"Bracket order: {len(signed_bracket['actions'])} actions in 1 tx")
    print(f"Bracket order IDs: {signed_bracket.get('order_ids')}")
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
    faucet_tag = next(iter(signed_faucet["actions"][0].keys()))
    print(f"Faucet action tag: {faucet_tag}")
    print()

    # 10. Sign user settings
    print("--- User Settings (Leverage) ---")
    signed_settings = signer.sign_user_settings([
        ("BTC-USD", 5.0),
        ("ETH-USD", 3.0)
    ])
    settings_tag = next(iter(signed_settings["actions"][0].keys()))
    print(f"Settings action tag: {settings_tag}")
    print()

    # 11. Sign oracle price update(s)
    print("--- Oracle Prices (px) ---")
    signed_oracle = signer.sign_oracle_prices([
        (1704067200000000000, "BTC-USD", 102500.0),
        (1704067200000000000, "ETH-USD", 3250.0),
    ])
    oracle_tag = next(iter(signed_oracle["actions"][0].keys()))
    print(f"Oracle action tag: {oracle_tag}")
    print()

    # 12. Sign Pyth oracle batch update
    print("--- Pyth Oracle (o) ---")
    signed_pyth = signer.sign_pyth_oracle([
        (1704067200000000000, 0, 10250000000000, -8),
        (1704067200000000000, 1, 325000000000, -8),
    ])
    pyth_tag = next(iter(signed_pyth["actions"][0].keys()))
    print(f"Pyth action tag: {pyth_tag}")
    print()

    # 13. Sign whitelist faucet admin action
    print("--- Whitelist Faucet ---")
    target = Keypair().pubkey
    signed_whitelist = signer.sign_whitelist_faucet(target, True)
    whitelist_tag = next(iter(signed_whitelist["actions"][0].keys()))
    print(f"Whitelist action tag: {whitelist_tag}")

    print("\n=== Done ===")


if __name__ == "__main__":
    main()
