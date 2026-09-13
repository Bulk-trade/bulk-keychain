import unittest
import os
import json
import base64
from bulk_keychain import Keypair, Signer, prepare_order, prepare_wallet_message, finalize_wallet_message

class WalletTests(unittest.TestCase):
    def test_all_modes(self):
        fixtures = []
        key = Keypair(bytes([1]) * 32)
        signer = Signer(key, "devnet")
        for kind in ("limit", "market"):
            order = {"type":"order", "symbol":"BTC-USD", "is_buy":True, "price":100.0,"size":1.0,
                     "order_type":{"type":kind,"tif":"GTC","is_market":True,"trigger_px":0}}
            if kind == "market": order["slippage"] = 100
            raw = prepare_order(order,"devnet",key.pubkey,nonce=9007199254740993)
            for mode in ("raw", "base58", "offchain"):
                wallet = prepare_wallet_message(raw,mode)
                signature = signer.sign_bytes(wallet["message_bytes"])
                self.assertEqual(finalize_wallet_message(wallet,signature)["actions"],raw["actions"])
                self.assertEqual(signer.sign_wallet_prepared(wallet)["signature"],signature)
                transaction = finalize_wallet_message(wallet,signature)
                transaction["nonce"] = str(transaction["nonce"])
                fixtures.append({"name":kind,"mode":mode,"messageBytesBase64":base64.b64encode(wallet["message_bytes"]).decode(),
                                 "clearSignMessage":wallet["clear_sign_message"],"transaction":transaction})
                self.assertEqual(wallet["prepared"]["nonce"],9007199254740993)
                with self.assertRaises(ValueError): finalize_wallet_message({**wallet,"message_bytes":bytes(1)},signature)
                with self.assertRaises(ValueError): Signer(key,"mainnet").sign_wallet_prepared(wallet)
        with self.assertRaises(ValueError): prepare_wallet_message(raw,"invalid")
        if os.getenv("BULK_KEYCHAIN_WALLET_FIXTURES"):
            with open(os.environ["BULK_KEYCHAIN_WALLET_FIXTURES"], "w") as output:
                json.dump(fixtures, output, indent=2)

if __name__ == "__main__": unittest.main()
