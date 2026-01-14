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
)

__all__ = [
    "Keypair",
    "Signer",
    "random_hash",
    "current_timestamp",
    "validate_pubkey",
    "validate_hash",
]

__version__ = "0.1.0"
