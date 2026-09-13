const { test } = require('node:test');
const assert = require('node:assert/strict');
const { NativeKeypair, buildDepositInstruction, buildWithdrawIntentInstruction } = require('../index.js');
const owner = NativeKeypair.fromBytes(Buffer.alloc(32, 1)).pubkey;

test('Solana builders preserve u64 amounts and account permissions', () => {
  const deposit = buildDepositInstruction(owner, '18446744073709551615');
  assert.ok(Buffer.isBuffer(deposit.data));
  assert.equal(deposit.data[0], 2);
  assert.equal(deposit.data.readBigUInt64LE(1), 18446744073709551615n);
  assert.equal(deposit.accounts.length, 6);
  assert.deepEqual(deposit.accounts[0], { pubkey: owner, isSigner: true, isWritable: true });
  assert.equal(deposit.accounts[1].isWritable, true);
  const intent = buildWithdrawIntentInstruction(owner, '9007199254740993');
  assert.equal(intent.data[0], 4);
  assert.equal(intent.data.readBigUInt64LE(1), 9007199254740993n);
  assert.equal(intent.accounts.length, 5);
  assert.equal(intent.accounts[1].isWritable, false);
  assert.equal(intent.programId, deposit.programId);
  assert.equal(intent.accounts[1].pubkey, deposit.accounts[1].pubkey);
});

test('Solana builders reject imprecise or malformed amount inputs', () => {
  for (const build of [buildDepositInstruction, buildWithdrawIntentInstruction]) {
    for (const amount of [9007199254740992, '1.5', '-1', '', '18446744073709551616']) {
      assert.throws(() => build(owner, amount));
    }
    assert.throws(() => build('invalid-owner', '1'));
  }
});
