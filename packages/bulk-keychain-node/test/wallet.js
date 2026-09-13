const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const n = require('../index.js');
const key = new n.NativeKeypair(Buffer.alloc(32,1));
const signer = new n.NativeSigner(key,'devnet');
const opts={signatureDomain:'devnet',account:key.pubkey,nonce:'9007199254740993'};
const limit={type:'order',symbol:'BTC-USD',isBuy:true,price:100,size:1,orderType:{type:'limit',tif:'GTC'}};
const market={...limit,orderType:{type:'market',isMarket:true,triggerPx:0},slippage:100};
const fixtures=[];
test('wallet modes sign exactly returned bytes and retain raw payload',()=>{
 for(const [name,order] of [['limit',limit],['market',market]]) for(const mode of ['raw','base58','offchain']) {
  const raw=n.prepareOrder(order,opts);const wallet=n.prepareWalletMessage(raw,mode);
  const signature=signer.signBytes(wallet.messageBytes);
  const tx=n.finalizeWalletMessage(wallet,signature);
  assert.equal(signer.signWalletPrepared(wallet).signature,signature);
  assert.equal(tx.nonce,opts.nonce);assert.equal(tx.actions,raw.actions);
  if(mode==='raw') assert.deepEqual(wallet.messageBytes,raw.messageBytes);
  if(mode==='base58') assert.equal(wallet.messageBytes.toString(),raw.messageBase58);
  if(mode==='offchain') {assert.ok(wallet.clearSignMessage.includes('Bulk Exchange Transaction'));assert.equal(wallet.messageBytes.subarray(0,16).toString('hex'),'ff736f6c616e61206f6666636861696e');}
  fixtures.push({name,mode,messageBytesBase64:wallet.messageBytes.toString('base64'),clearSignMessage:wallet.clearSignMessage??null,transaction:{...tx,actions:JSON.parse(tx.actions)}});
  assert.throws(()=>n.finalizeWalletMessage({...wallet,messageBytes:Buffer.alloc(1)},signature));
  assert.throws(()=>new n.NativeSigner(key,'mainnet').signWalletPrepared(wallet));
 }
 if(process.env.BULK_KEYCHAIN_WALLET_FIXTURES) fs.writeFileSync(process.env.BULK_KEYCHAIN_WALLET_FIXTURES,JSON.stringify(fixtures,null,2));
});
test('offchain unsupported actions and invalid modes reject',()=>{
 const raw=n.prepareOrder({...limit,onFill:{actions:[market]}},opts);
 assert.throws(()=>n.prepareWalletMessage(raw,'offchain'),/unsupported/);
 assert.throws(()=>n.prepareWalletMessage(raw,'invalid'));
 assert.ok(n.prepareWalletMessage(raw,'raw'));
});
