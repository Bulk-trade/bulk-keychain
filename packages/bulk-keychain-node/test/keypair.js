const { test } = require('node:test');
const assert = require('node:assert/strict');
const { NativeKeypair, NativeSigner } = require('../index.js');
const fixture = NativeKeypair.fromBytes(Buffer.alloc(32, 1));

test('constructors import supplied keys and sign with them', () => {
  for (const supplied of [fixture.toBase58(), Buffer.alloc(32, 1), new Uint8Array(fixture.toBytes())]) {
    const imported = new NativeKeypair(supplied);
    assert.deepEqual(imported.toBytes(), fixture.toBytes());
    assert.equal(new NativeSigner(imported, 'devnet').signBytes(Buffer.from('fixture')),
      new NativeSigner(fixture, 'devnet').signBytes(Buffer.from('fixture')));
  }
  assert.notEqual(new NativeKeypair().pubkey, new NativeKeypair().pubkey);
});
test('malformed keys fail instead of generating or repairing a key', () => {
  const corrupt = Buffer.from(fixture.toBytes()); corrupt[63] ^= 1;
  for (const supplied of [null, 1, {}, [], '', 'not-base58!', Buffer.alloc(31), corrupt]) {
    assert.throws(() => new NativeKeypair(supplied));
  }
  assert.throws(() => NativeKeypair.fromBytes(corrupt));
});
