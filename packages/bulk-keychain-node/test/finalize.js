const {test} = require('node:test');
const assert = require('node:assert/strict');
const {NativeKeypair, NativeSigner, prepareOrder, finalizePreparedTransaction} = require('../index.js');
const key = new NativeKeypair(Buffer.alloc(32, 1));
const signer = new NativeSigner(key, 'devnet');
const prepared = prepareOrder({type:'order',symbol:'BTC-USD',isBuy:true,price:100,size:1,orderType:{type:'limit',tif:'GTC'}},
  {signatureDomain:'devnet',account:key.pubkey,nonce:'42'});
const signature = signer.signBytes(prepared.messageBytes);
test('finalization verifies signature and prepared metadata', () => {
  assert.equal(finalizePreparedTransaction(prepared, signature).signature, signature);
  assert.equal(signer.signPrepared(prepared).signature, signature);
  for (const signature of ['bad', '1'.repeat(64)]) assert.throws(() => finalizePreparedTransaction(prepared, signature));
  const other = new NativeKeypair(Buffer.alloc(32, 2)).pubkey;
  for (const changes of [{account:other}, {signer:other}, {nonce:'43'}, {actions:'invalid-json'}, {messageBytes:Buffer.alloc(0)}]) {
    assert.throws(() => finalizePreparedTransaction({...prepared,...changes}, signature));
  }
});
