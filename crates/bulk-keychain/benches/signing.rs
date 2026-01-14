//! Benchmarks for signing performance

use bulk_keychain::{Keypair, Order, OrderItem, Signer, TimeInForce};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn bench_single_sign(c: &mut Criterion) {
    let keypair = Keypair::generate();
    let mut signer = Signer::new(keypair);

    let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);

    c.bench_function("sign_single_order", |b| {
        b.iter(|| {
            let result = signer.sign_order(vec![black_box(order.clone()).into()], Some(1234567890));
            black_box(result).unwrap()
        })
    });
}

fn bench_batch_sign(c: &mut Criterion) {
    let keypair = Keypair::generate();
    let signer = Signer::new(keypair);

    // Create 100 order batches
    let batches: Vec<Vec<OrderItem>> = (0..100)
        .map(|i| {
            vec![Order::limit("BTC-USD", i % 2 == 0, 100000.0 + i as f64, 0.1, TimeInForce::Gtc).into()]
        })
        .collect();

    let mut group = c.benchmark_group("batch_signing");
    group.throughput(Throughput::Elements(100));

    group.bench_function("sign_100_orders_parallel", |b| {
        b.iter(|| {
            let batches_clone = batches.clone();
            let result = signer.sign_orders_batch(black_box(batches_clone), Some(1000000));
            black_box(result).unwrap()
        })
    });

    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    use bulk_keychain::serialize::WincodeSerializer;
    use bulk_keychain::types::*;

    let order = Order::limit("BTC-USD", true, 100000.0, 0.1, TimeInForce::Gtc);
    let action = Action::Order {
        orders: vec![order.into()],
    };
    let account = Keypair::generate().pubkey();
    let signer_key = account;

    c.bench_function("serialize_order", |b| {
        let mut serializer = WincodeSerializer::new();
        b.iter(|| {
            serializer.reset();
            serializer.serialize_for_signing(
                black_box(&action),
                1234567890,
                black_box(&account),
                black_box(&signer_key),
            );
            black_box(serializer.as_bytes())
        })
    });
}

fn bench_keypair_generation(c: &mut Criterion) {
    c.bench_function("generate_keypair", |b| {
        b.iter(|| {
            let keypair = Keypair::generate();
            black_box(keypair)
        })
    });
}

criterion_group!(
    benches,
    bench_single_sign,
    bench_batch_sign,
    bench_serialization,
    bench_keypair_generation,
);
criterion_main!(benches);
