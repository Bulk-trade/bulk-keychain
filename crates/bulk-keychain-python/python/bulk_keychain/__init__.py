"""
bulk-keychain: A simple high perf signing lib for BULK txns

This package provides fast, native transaction signing using Rust bindings.

Quick Start
-----------
>>> from bulk_keychain import Keypair, Signer
>>>
>>> # Generate a keypair
>>> keypair = Keypair()
>>> print(keypair.pubkey)
>>>
>>> # Create a signer
>>> signer = Signer(keypair)
>>>
>>> # Sign a single order
>>> signed = signer.sign({
...     "type": "order",
...     "symbol": "BTC-USD",
...     "is_buy": True,
...     "price": 100000.0,
...     "size": 0.1,
...     "order_type": {"type": "limit", "tif": "GTC"}
... })
"""

from bulk_keychain._native import (
    Keypair,
    Signer,
    random_hash,
    current_timestamp,
    validate_pubkey,
    validate_hash,
    compute_order_id,
    # External wallet support
    py_prepare_order as prepare_order,
    py_prepare_all_orders as prepare_all_orders,
    py_prepare_order_group as prepare_order_group,
    py_prepare_agent_wallet_auth as prepare_agent_wallet,
    py_prepare_faucet_request as prepare_faucet,
    py_finalize_transaction as finalize_transaction,
)

__all__ = [
    "Keypair",
    "Signer",
    "random_hash",
    "current_timestamp",
    "validate_pubkey",
    "validate_hash",
    "compute_order_id",
    # External wallet support
    "prepare_order",
    "prepare_all_orders",
    "prepare_order_group",
    "prepare_agent_wallet",
    "prepare_faucet",
    "finalize_transaction",
]

__version__ = "0.1.4"
