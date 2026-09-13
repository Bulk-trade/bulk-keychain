import unittest
from bulk_keychain import Keypair, Signer, prepare_order, finalize_transaction

class FinalizationTests(unittest.TestCase):
    def test_signature_and_metadata(self):
        key = Keypair(bytes([1]) * 32)
        signer = Signer(key, "devnet")
        prepared = prepare_order({"type":"order", "symbol":"BTC-USD", "is_buy":True,
                                  "price":100.0, "size":1.0}, "devnet", key.pubkey, nonce=42)
        signature = signer.sign_bytes(prepared["message_bytes"])
        self.assertEqual(finalize_transaction(prepared, signature)["signature"], signature)
        self.assertEqual(signer.sign_prepared(prepared)["signature"], signature)
        for invalid in ("bad", "1" * 64):
            with self.assertRaises(ValueError):
                finalize_transaction(prepared, invalid)
        other = Keypair(bytes([2]) * 32).pubkey
        for changes in ({"account":other}, {"signer":other}, {"nonce":43}, {"actions":"bad"}, {"message_bytes":bytes()}):
            with self.assertRaises(ValueError):
                finalize_transaction({**prepared, **changes}, signature)

if __name__ == "__main__":
    unittest.main()
