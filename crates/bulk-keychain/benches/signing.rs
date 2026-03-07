//! Benchmarks for signing performance.

use bulk_keychain::{Keypair, Order, OrderItem, Signer, TimeInForce};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use sha2::{Digest, Sha256};

const BATCH_SIZE: usize = 256;
const GROUP_SIZE: usize = 3;
const SCALE_1E8: u64 = 100_000_000;

#[inline]
fn make_order(i: usize) -> OrderItem {
    Order::limit(
        "BTC-USD",
        i.is_multiple_of(2),
        100000.0 + i as f64,
        0.1,
        TimeInForce::Gtc,
    )
    .into()
}

fn bench_sign_single(c: &mut Criterion) {
    let order = make_order(0);

    let mut signer_with_order_id = Signer::new(Keypair::generate());
    let mut signer_without_order_id = Signer::new(Keypair::generate()).without_order_id();

    let mut group = c.benchmark_group("sign_single");
    group.throughput(Throughput::Elements(1));

    group.bench_function("with_order_id", |b| {
        b.iter(|| {
            let tx = signer_with_order_id
                .sign(black_box(order.clone()), Some(1234567890))
                .unwrap();
            black_box(tx)
        })
    });

    group.bench_function("without_order_id", |b| {
        b.iter(|| {
            let tx = signer_without_order_id
                .sign(black_box(order.clone()), Some(1234567890))
                .unwrap();
            black_box(tx)
        })
    });

    group.finish();
}

fn bench_sign_all(c: &mut Criterion) {
    let orders: Vec<OrderItem> = (0..BATCH_SIZE).map(make_order).collect();

    let signer_with_order_id = Signer::new(Keypair::generate());
    let signer_without_order_id = Signer::new(Keypair::generate()).without_order_id();

    let mut group = c.benchmark_group("sign_all_parallel");
    group.throughput(Throughput::Elements(BATCH_SIZE as u64));

    group.bench_function("with_order_id", |b| {
        b.iter(|| {
            let txs = signer_with_order_id
                .sign_all(black_box(orders.clone()), Some(1000000))
                .unwrap();
            black_box(txs)
        })
    });

    group.bench_function("without_order_id", |b| {
        b.iter(|| {
            let txs = signer_without_order_id
                .sign_all(black_box(orders.clone()), Some(1000000))
                .unwrap();
            black_box(txs)
        })
    });

    group.finish();
}

fn bench_sign_group(c: &mut Criterion) {
    let bracket: Vec<OrderItem> = vec![
        Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc).into(),
        Order::limit("BTC-USD", false, 99000.0, 0.1, TimeInForce::Gtc).into(),
        Order::limit("BTC-USD", false, 110000.0, 0.1, TimeInForce::Gtc).into(),
    ];

    let mut signer_default = Signer::new(Keypair::generate());
    let mut signer_with_batch_ids = Signer::new(Keypair::generate()).with_batch_order_ids();

    let mut group = c.benchmark_group("sign_group_atomic");
    group.throughput(Throughput::Elements(GROUP_SIZE as u64));

    group.bench_function("without_batch_order_ids", |b| {
        b.iter(|| {
            let tx = signer_default
                .sign_group(black_box(bracket.clone()), Some(1234567890))
                .unwrap();
            black_box(tx)
        })
    });

    group.bench_function("with_batch_order_ids", |b| {
        b.iter(|| {
            let tx = signer_with_batch_ids
                .sign_group(black_box(bracket.clone()), Some(1234567890))
                .unwrap();
            black_box(tx)
        })
    });

    group.finish();
}

#[inline]
fn parse_scaled_1e8(value: &str) -> u64 {
    let bytes = value.as_bytes();
    let mut i = 0usize;
    let mut int_part: u64 = 0;
    while i < bytes.len() && bytes[i] != b'.' {
        let digit = bytes[i].wrapping_sub(b'0');
        if digit <= 9 {
            int_part = int_part.saturating_mul(10).saturating_add(digit as u64);
        }
        i += 1;
    }

    if i == bytes.len() {
        return int_part.saturating_mul(SCALE_1E8);
    }

    i += 1; // skip '.'
    let mut frac_part: u64 = 0;
    let mut frac_digits = 0usize;
    while i < bytes.len() && frac_digits < 8 {
        let digit = bytes[i].wrapping_sub(b'0');
        if digit <= 9 {
            frac_part = frac_part * 10 + digit as u64;
            frac_digits += 1;
        } else {
            break;
        }
        i += 1;
    }

    while frac_digits < 8 {
        frac_part *= 10;
        frac_digits += 1;
    }

    // Round half-up using the 9th fractional digit if present.
    if i < bytes.len() {
        let next = bytes[i].wrapping_sub(b'0');
        if next >= 5 && next <= 9 {
            frac_part = frac_part.saturating_add(1);
            if frac_part == SCALE_1E8 {
                frac_part = 0;
                int_part = int_part.saturating_add(1);
            }
        }
    }

    int_part.saturating_mul(SCALE_1E8).saturating_add(frac_part)
}

#[inline]
fn hash_oid_fields(
    symbol: &str,
    is_buy: bool,
    px_scaled: u64,
    sz_scaled: u64,
    tif: u32,
    reduce_only: bool,
    account: &[u8; 32],
    nonce: u64,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(1u32.to_le_bytes());
    hasher.update((symbol.len() as u64).to_le_bytes());
    hasher.update(symbol.as_bytes());
    hasher.update([if is_buy { 1 } else { 0 }]);
    hasher.update(px_scaled.to_le_bytes());
    hasher.update(sz_scaled.to_le_bytes());
    hasher.update(tif.to_le_bytes());
    hasher.update([if reduce_only { 1 } else { 0 }]);
    hasher.update(account);
    hasher.update(nonce.to_le_bytes());
    hasher.finalize().into()
}

fn bench_oid_john_vs_junbug(c: &mut Criterion) {
    let symbol = "BTC-USD";
    let is_buy = true;
    let tif = 0u32;
    let reduce_only = false;
    let account = [7u8; 32];

    // Same semantic values represented as float vs string.
    let px_f64 = 68495.12345678_f64;
    let sz_f64 = 0.014613_f64;
    let nonce_u64 = 1_772_817_472_617_u64;

    let px_str = "68495.12345678";
    let sz_str = "0.014613";
    let nonce_str = "1772817472617";

    // JunBug-style: normalize once at boundary, hash from canonical ints.
    let px_pre = parse_scaled_1e8(px_str);
    let sz_pre = parse_scaled_1e8(sz_str);
    let nonce_pre = nonce_str.parse::<u64>().unwrap();

    let mut group = c.benchmark_group("john_vs_junbug_oid");
    group.throughput(Throughput::Elements(1));

    // John-style: field-by-field with f64 round() to fixed-point.
    group.bench_function("john_field_by_field_f64_round", |b| {
        b.iter(|| {
            let px_scaled = (black_box(px_f64) * SCALE_1E8 as f64).round() as u64;
            let sz_scaled = (black_box(sz_f64) * SCALE_1E8 as f64).round() as u64;
            let digest = hash_oid_fields(
                symbol,
                is_buy,
                px_scaled,
                sz_scaled,
                tif,
                reduce_only,
                &account,
                nonce_u64,
            );
            black_box(digest)
        })
    });

    // JunBug-style (naive): accept strings and parse each request before hashing.
    group.bench_function("junbug_string_boundary_parse_each_call", |b| {
        b.iter(|| {
            let px_scaled = parse_scaled_1e8(black_box(px_str));
            let sz_scaled = parse_scaled_1e8(black_box(sz_str));
            let nonce = black_box(nonce_str).parse::<u64>().unwrap();
            let digest = hash_oid_fields(
                symbol,
                is_buy,
                px_scaled,
                sz_scaled,
                tif,
                reduce_only,
                &account,
                nonce,
            );
            black_box(digest)
        })
    });

    // JunBug-style (optimized): parse once at boundary, then hash canonical ints in hot path.
    group.bench_function("junbug_string_boundary_preparsed_hot_path", |b| {
        b.iter(|| {
            let digest = hash_oid_fields(
                symbol,
                is_buy,
                px_pre,
                sz_pre,
                tif,
                reduce_only,
                &account,
                nonce_pre,
            );
            black_box(digest)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sign_single,
    bench_sign_all,
    bench_sign_group,
    bench_oid_john_vs_junbug
);
criterion_main!(benches);
