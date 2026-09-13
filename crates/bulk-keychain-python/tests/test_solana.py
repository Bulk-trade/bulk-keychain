import unittest

from bulk_keychain import Keypair, build_deposit_instruction, build_withdraw_intent_instruction

OWNER = Keypair.from_bytes(bytes([1]) * 32).pubkey


class SolanaInstructionTests(unittest.TestCase):
    def test_exact_amounts_and_account_permissions(self):
        deposit = build_deposit_instruction(OWNER, 2**64 - 1)
        intent = build_withdraw_intent_instruction(OWNER, 2**53 + 1)
        self.assertEqual(deposit["data"], bytes([2]) + (2**64 - 1).to_bytes(8, "little"))
        self.assertEqual(intent["data"], bytes([4]) + (2**53 + 1).to_bytes(8, "little"))
        self.assertEqual(len(deposit["accounts"]), 6)
        self.assertEqual(len(intent["accounts"]), 5)
        self.assertTrue(deposit["accounts"][1]["is_writable"])
        self.assertFalse(intent["accounts"][1]["is_writable"])
        self.assertEqual(deposit["accounts"][0]["pubkey"], OWNER)

    def test_invalid_inputs(self):
        for build in (build_deposit_instruction, build_withdraw_intent_instruction):
            for amount in (-1, 2**64, 1.5):
                with self.assertRaises((OverflowError, TypeError, ValueError)):
                    build(OWNER, amount)
            with self.assertRaises(ValueError):
                build("invalid-owner", 1)


if __name__ == "__main__":
    unittest.main()
