// Build with wasm-pack --target nodejs, then set BULK_KEYCHAIN_WASM_MODULE to the generated .js.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const w = require(process.env.BULK_KEYCHAIN_WASM_MODULE || '../pkg/bulk_keychain_wasm.js');
const key = new w.WasmKeypair(new Uint8Array(32).fill(1));
const signer = new w.WasmSigner(key, 'devnet');
const nonce = '9007199254740993';
const order = {type:'order',symbol:'BTC-USD',isBuy:true,price:100,size:1,orderType:{type:'limit',tif:'GTC'}};
const market = {...order, orderType:{type:'market',isMarket:true,triggerPx:0},slippage:100};
const fixtures = [];
function roundtrip(name, transaction) {
  const parsed = JSON.parse(JSON.stringify(transaction));
  assert.deepEqual(parsed.actions, transaction.actions);
  assert.equal(parsed.nonce, nonce);
  assert.equal(typeof parsed.nonce, 'string');
  assert.equal(Object.getPrototypeOf(parsed.actions[0]), Object.prototype);
  assert.ok(Object.keys(parsed.actions[0]).length > 0);
  fixtures.push({name, transaction:parsed});
  return parsed;
}
test('signed, grouped and finalized actions serialize as JSON objects', () => {
  assert.equal(roundtrip('limit', signer.sign(order, nonce)).actions[0].l.c, 'BTC-USD');
  roundtrip('group', signer.signGroup([order, market], nonce));
  for (const [i, tx] of signer.signAll([order], nonce).entries()) roundtrip('all-' + i, tx);
  const onFill = {...order,onFill:{actions:[market]}};
  const prepared = w.prepareOrder(onFill, {signatureDomain:'devnet',account:key.pubkey,nonce});
  assert.deepEqual(JSON.parse(JSON.stringify(prepared.actions)), prepared.actions);
  assert.equal(prepared.actions[0].of.actions[0].m.slippage, 100);
  const signature = signer.signBytes(prepared.messageBytes);
  roundtrip('finalized-onfill', prepared.finalize(signature));
  roundtrip('sign-prepared', signer.signPrepared(prepared));
  roundtrip('multisig', signer.signMultisigPropose(key.pubkey, [{l:{c:'BTC-USD',b:true,px:100,sz:1,tif:'GTC',r:false,i:false}}], nonce));
  if (process.env.BULK_KEYCHAIN_JSON_FIXTURES) fs.writeFileSync(process.env.BULK_KEYCHAIN_JSON_FIXTURES, JSON.stringify(fixtures, null, 2));
});
test('typed Solana instruction byte arrays and account objects stay unchanged', () => {
  const instruction = w.buildDepositInstruction(key.pubkey, '9007199254740993');
  assert.ok(Array.isArray(instruction.data));
  assert.equal(instruction.data.length, 9);
  assert.equal(typeof instruction.accounts[0].isSigner, 'boolean');
  assert.deepEqual(JSON.parse(JSON.stringify(instruction)), instruction);
});
