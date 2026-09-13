const { test } = require('node:test');
const assert = require('node:assert/strict');
const { NativeKeypair, exportDepositTransaction, exportWithdrawIntentTransaction } = require('../index.js');
const owner = NativeKeypair.fromBytes(Buffer.alloc(32, 1)).pubkey;
const blockhash = NativeKeypair.fromBytes(Buffer.alloc(32, 2)).pubkey;

test('exports base64 unsigned Solana transactions with exact amounts', () => {
  for (const exportTransaction of [exportDepositTransaction, exportWithdrawIntentTransaction]) {
    const encoded = exportTransaction(owner, '18446744073709551615', blockhash);
    const bytes = Buffer.from(encoded, 'base64');
    assert.equal(bytes.toString('base64'), encoded);
    assert.equal(bytes[0], 1);
    assert.deepEqual(bytes.subarray(1, 65), Buffer.alloc(64));
    assert.equal(bytes[65], 1);
    assert.equal(encoded, exportTransaction(owner, '18446744073709551615', blockhash));
    assert.notEqual(encoded, exportTransaction(owner, '18446744073709551614', blockhash));
    assert.throws(() => exportTransaction(owner, '1', 'invalid'));
    assert.throws(() => exportTransaction(owner, 1, blockhash));
    assert.throws(() => exportTransaction(owner, '18446744073709551616', blockhash));
  }
  assert.notEqual(exportDepositTransaction(owner, '1', blockhash), exportWithdrawIntentTransaction(owner, '1', blockhash));
});
