//! Benchmarks for signing performance.

use bulk_keychain::{Keypair, Order, OrderItem, Signer, TimeInForce};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

const BATCH_SIZE: usize = 256;
const GROUP_SIZE: usize = 3;

#[inline]
fn make_order(i: usize) -> OrderItem {
    Order::limit(
        "BTC-USD",
        i % 2 == 0,
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

criterion_group!(benches, bench_sign_single, bench_sign_all, bench_sign_group);
criterion_main!(benches);
