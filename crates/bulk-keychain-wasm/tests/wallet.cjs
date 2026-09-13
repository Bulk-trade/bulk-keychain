// Build wasm-pack --target nodejs and point BULK_KEYCHAIN_WASM_MODULE to its generated module.
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const w=require(process.env.BULK_KEYCHAIN_WASM_MODULE || '../pkg/bulk_keychain_wasm.js');
const key=new w.WasmKeypair(new Uint8Array(32).fill(1));
const signer=new w.WasmSigner(key,'devnet');
const opts={signatureDomain:'devnet',account:key.pubkey,nonce:'9007199254740993'};
const limit={type:'order',symbol:'BTC-USD',isBuy:true,price:100,size:1,orderType:{type:'limit',tif:'GTC'}};
const market={...limit,orderType:{type:'market',isMarket:true,triggerPx:0},slippage:100};
test('wallet modes preserve bytes, signatures and live wrapper references',()=>{
 const fixtures=[];
 for(const [name,order] of [['limit',limit],['market',market]]) for(const mode of ['raw','base58','offchain']) {
  const raw=w.prepareOrder(order,opts);const wallet=w.prepareWalletMessage(raw,mode);
  assert.equal(raw.nonce,opts.nonce);assert.equal(wallet.prepared.nonce,opts.nonce);
  const signature=signer.signBytes(wallet.messageBytes);
  const signed=w.finalizeWalletMessage(wallet,signature);
  assert.equal(signer.signWalletPrepared(wallet).signature,signature);
  assert.equal(wallet.signatureMode,mode);
  assert.deepEqual(signed.actions,raw.actions);
  const tampered=wallet.messageBytes;tampered[0]^=1;
  assert.throws(()=>w.finalizeWalletMessage(wallet,signer.signBytes(tampered)));
  assert.throws(()=>new w.WasmSigner(key,'mainnet').signWalletPrepared(wallet));
  fixtures.push({name,mode,messageBytesBase64:Buffer.from(wallet.messageBytes).toString('base64'),clearSignMessage:wallet.clearSignMessage??null,transaction:JSON.parse(JSON.stringify(signed))});
 }
 if(process.env.BULK_KEYCHAIN_WALLET_FIXTURES) fs.writeFileSync(process.env.BULK_KEYCHAIN_WALLET_FIXTURES,JSON.stringify(fixtures,null,2));
});
test('unsupported offchain and invalid modes fail',()=>{
 const raw=w.prepareOrder({...limit,onFill:{actions:[market]}},opts);
 assert.throws(()=>w.prepareWalletMessage(raw,'offchain'),/unsupported/);
 assert.throws(()=>w.prepareWalletMessage(raw,'invalid'));
 assert.ok(w.prepareWalletMessage(raw,'raw'));
});
