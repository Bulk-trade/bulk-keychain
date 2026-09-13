import base64
import unittest
from bulk_keychain import Keypair, export_deposit_transaction, export_withdraw_intent_transaction

class SolanaExportTests(unittest.TestCase):
    def test_unsigned_base64_and_exact_amounts(self):
        owner = Keypair(bytes([1]) * 32).pubkey
        blockhash = Keypair(bytes([2]) * 32).pubkey
        for export in (export_deposit_transaction, export_withdraw_intent_transaction):
            encoded = export(owner, 2**64 - 1, blockhash)
            raw = base64.b64decode(encoded, validate=True)
            self.assertEqual(raw[:65], bytes([1]) + bytes(64))
            self.assertEqual(raw[65], 1)
            self.assertEqual(encoded, export(owner, 2**64 - 1, blockhash))
            self.assertNotEqual(encoded, export(owner, 2**64 - 2, blockhash))
            with self.assertRaises(ValueError):
                export(owner, 1, "invalid")
            with self.assertRaises(OverflowError):
                export(owner, 2**64, blockhash)

if __name__ == "__main__":
    unittest.main()
