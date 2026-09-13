import unittest
from bulk_keychain import Keypair, Signer

class KeypairImportTests(unittest.TestCase):
    def test_constructor_preserves_key_and_signature(self):
        fixture = Keypair.from_bytes(bytes([1]) * 32)
        for key in (fixture.to_base58(), bytes([1]) * 32, bytes(fixture.to_bytes())):
            imported = Keypair(key)
            self.assertEqual(imported.to_bytes(), fixture.to_bytes())
            self.assertEqual(Signer(imported, "devnet").sign_bytes(b"fixture"),
                             Signer(fixture, "devnet").sign_bytes(b"fixture"))
        self.assertNotEqual(Keypair().pubkey, Keypair().pubkey)

    def test_invalid_inputs_fail(self):
        corrupt = bytearray(Keypair(bytes([1]) * 32).to_bytes())
        corrupt[63] ^= 1
        for key in (None, 1, {}, [], "", "not-base58!", bytes(31), bytes(corrupt)):
            with self.assertRaises((TypeError, ValueError)):
                Keypair(key)
        with self.assertRaises(ValueError):
            Keypair.from_bytes(bytes(corrupt))

if __name__ == "__main__":
    unittest.main()
