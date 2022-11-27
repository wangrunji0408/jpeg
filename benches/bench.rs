use criterion::{criterion_group, criterion_main, Criterion};
use jpeg_labs::mcu::Block;

criterion_group!(benches, block);
criterion_main!(benches);

fn block(c: &mut Criterion) {
    let block = Block([0; 64]);
    c.bench_function("idct", |b| b.iter(|| block.idct()));
    c.bench_function("zigzag", |b| b.iter(|| block.zigzag()));
    c.bench_function("dequantize", |b| b.iter(|| block.dequantize(&[1; 64])));
}
