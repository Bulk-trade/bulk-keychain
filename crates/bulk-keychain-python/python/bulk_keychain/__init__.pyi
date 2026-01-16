"""Type stubs for bulk_keychain"""

from typing import Any, Literal, TypedDict, NotRequired

class OrderTypeLimit(TypedDict):
    type: Literal["limit"]
    tif: NotRequired[Literal["GTC", "IOC", "ALO"]]

class OrderTypeTrigger(TypedDict):
    type: Literal["trigger", "market"]
    is_market: NotRequired[bool]
    trigger_px: NotRequired[float]

OrderType = OrderTypeLimit | OrderTypeTrigger

class OrderItem(TypedDict):
    type: Literal["order"]
    symbol: str
    is_buy: bool
    price: float
    size: float
    reduce_only: NotRequired[bool]
    order_type: NotRequired[OrderType]
    client_id: NotRequired[str]

class CancelItem(TypedDict):
    type: Literal["cancel"]
    symbol: str
    order_id: str

class CancelAllItem(TypedDict):
    type: Literal["cancel_all"]
    symbols: NotRequired[list[str]]

OrderItemType = OrderItem | CancelItem | CancelAllItem

class SignedTransaction(TypedDict):
    action: dict[str, Any]
    account: str
    signer: str
    signature: str
    order_id: NotRequired[str]  # Pre-computed order ID (SHA256 of wincode bytes)

class Keypair:
    """Ed25519 keypair for signing transactions"""

    def __init__(self) -> None:
        """Generate a new random keypair"""
        ...

    @staticmethod
    def from_base58(s: str) -> "Keypair":
        """Create from base58-encoded secret key or full keypair"""
        ...

    @staticmethod
    def from_bytes(bytes: bytes) -> "Keypair":
        """Create from raw bytes (32-byte secret or 64-byte keypair)"""
        ...

    @property
    def pubkey(self) -> str:
        """Get the public key as base58 string"""
        ...

    def to_base58(self) -> str:
        """Get the full keypair as base58 (64 bytes)"""
        ...

    def to_bytes(self) -> bytes:
        """Get the full keypair as bytes (64 bytes)"""
        ...

    def secret_key(self) -> bytes:
        """Get the secret key as bytes (32 bytes)"""
        ...

class Signer:
    """High-performance transaction signer"""

    def __init__(self, keypair: Keypair) -> None:
        """Create a new signer from a keypair"""
        ...

    @staticmethod
    def from_base58(s: str) -> "Signer":
        """Create a signer from base58-encoded secret key"""
        ...

    @staticmethod
    def with_nonce_manager(
        keypair: Keypair,
        strategy: Literal["timestamp", "counter", "high_frequency"]
    ) -> "Signer":
        """Create a signer with nonce management"""
        ...

    @property
    def pubkey(self) -> str:
        """Get the signer's public key"""
        ...

    # ========================================================================
    # Simplified API
    # ========================================================================

    def sign(
        self,
        order: OrderItemType,
        nonce: int | None = None
    ) -> SignedTransaction:
        """Sign a single order/cancel/cancelAll - returns single transaction"""
        ...

    def sign_all(
        self,
        orders: list[OrderItemType],
        base_nonce: int | None = None
    ) -> list[SignedTransaction]:
        """Sign multiple orders - each becomes its own transaction (parallel)
        
        Optimized for HFT: each order gets independent confirmation/rejection.
        """
        ...

    def sign_group(
        self,
        orders: list[OrderItemType],
        nonce: int | None = None
    ) -> SignedTransaction:
        """Sign multiple orders atomically in ONE transaction
        
        Use for bracket orders (entry + stop loss + take profit).
        """
        ...

    # ========================================================================
    # Other signing methods
    # ========================================================================

    def sign_faucet(self, nonce: int | None = None) -> SignedTransaction:
        """Sign a faucet request (testnet only)"""
        ...

    def sign_agent_wallet(
        self,
        agent_pubkey: str,
        delete: bool,
        nonce: int | None = None
    ) -> SignedTransaction:
        """Sign agent wallet creation/deletion"""
        ...

    def sign_user_settings(
        self,
        max_leverage: list[tuple[str, float]],
        nonce: int | None = None
    ) -> SignedTransaction:
        """Sign user settings update"""
        ...

    # ========================================================================
    # Legacy methods (deprecated)
    # ========================================================================

    def sign_order(
        self,
        orders: list[OrderItemType],
        nonce: int | None = None
    ) -> SignedTransaction:
        """Deprecated: Use sign(), sign_all(), or sign_group() instead"""
        ...

    def sign_orders_batch(
        self,
        batches: list[list[OrderItemType]],
        base_nonce: int | None = None
    ) -> list[SignedTransaction]:
        """Deprecated: Use sign_all() instead"""
        ...

def random_hash() -> str:
    """Generate a random hash (for client order IDs)"""
    ...

def current_timestamp() -> int:
    """Get current timestamp in milliseconds"""
    ...

def validate_pubkey(s: str) -> bool:
    """Validate a base58-encoded public key"""
    ...

def validate_hash(s: str) -> bool:
    """Validate a base58-encoded hash"""
    ...

def compute_order_id(wincode_bytes: bytes) -> str:
    """Compute order ID from wincode bytes
    
    This computes SHA256(wincode_bytes), which matches BULK's server-side
    order ID generation. Useful if you're serializing transactions yourself.
    """
    ...

# ============================================================================
# External Wallet Support - Prepare/Finalize API
# ============================================================================

class PreparedMessage(TypedDict):
    """Message prepared for external wallet signing"""
    message_bytes: bytes      # Raw bytes to pass to wallet.signMessage()
    message_base58: str       # Base58 encoded message
    message_base64: str       # Base64 encoded message
    message_hex: str          # Hex encoded message
    order_id: str             # Pre-computed order ID
    action: dict[str, Any]    # Action JSON for API
    account: str              # Account public key (base58)
    signer: str               # Signer public key (base58)
    nonce: int                # Nonce used

def py_prepare_order(
    order: OrderItemType,
    account: str,
    signer: str | None = None,
    nonce: int | None = None
) -> PreparedMessage:
    """Prepare a single order for external wallet signing
    
    Use this when you don't have access to the private key and need
    to sign with an external wallet.
    
    Example:
        prepared = py_prepare_order(order, "account_pubkey")
        signature = wallet.sign_message(prepared["message_bytes"])
        signed = py_finalize_transaction(prepared, signature)
    """
    ...

def py_prepare_all_orders(
    orders: list[OrderItemType],
    account: str,
    signer: str | None = None,
    base_nonce: int | None = None
) -> list[PreparedMessage]:
    """Prepare multiple orders - each becomes its own transaction (parallel)"""
    ...

def py_prepare_order_group(
    orders: list[OrderItemType],
    account: str,
    signer: str | None = None,
    nonce: int | None = None
) -> PreparedMessage:
    """Prepare multiple orders as ONE atomic transaction
    
    Use for bracket orders (entry + stop loss + take profit).
    """
    ...

def py_prepare_agent_wallet_auth(
    agent_pubkey: str,
    delete: bool,
    account: str,
    signer: str | None = None,
    nonce: int | None = None
) -> PreparedMessage:
    """Prepare agent wallet creation for external signing"""
    ...

def py_prepare_faucet_request(
    account: str,
    signer: str | None = None,
    nonce: int | None = None
) -> PreparedMessage:
    """Prepare faucet request for external signing"""
    ...

def py_finalize_transaction(
    prepared: PreparedMessage,
    signature: str
) -> SignedTransaction:
    """Finalize a prepared message with a signature from an external wallet
    
    Args:
        prepared: PreparedMessage dict from prepare_* functions
        signature: Base58-encoded signature from wallet
    
    Returns:
        SignedTransaction dict ready for API submission
    """
    ...
