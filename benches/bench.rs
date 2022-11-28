use criterion::{criterion_group, criterion_main, Criterion};
use jpeg_labs::{
    mcu::{Block, Mcu},
    start_of_frame_0::{ComponentInfo, StartOfFrameInfo},
};

criterion_group!(benches, block, mcu);
criterion_main!(benches);

fn block(c: &mut Criterion) {
    let block = Block([0; 64]);
    c.bench_function("idct", |b| b.iter(|| block.idct()));
    c.bench_function("zigzag", |b| b.iter(|| block.zigzag()));
    c.bench_function("dequantize", |b| b.iter(|| block.dequantize(&[1; 64])));
    c.bench_function("upsample", |b| b.iter(|| block.upsample_2x2(0, 0)));
}

fn mcu(c: &mut Criterion) {
    let mcu = Mcu {
        blocks: vec![Block([0; 64]); 6],
    };
    let s2 = ComponentInfo {
        horizontal_sampling: 2,
        vertical_sampling: 2,
        quant_table_id: 0,
    };
    let s1 = ComponentInfo {
        horizontal_sampling: 1,
        vertical_sampling: 1,
        quant_table_id: 1,
    };
    let sof = StartOfFrameInfo {
        precision: 8,
        height: 1080,
        width: 1920,
        component_infos: [s2, s1, s1],
        max_horizontal_sampling: 2,
        max_vertical_sampling: 2,
    };
    c.bench_function("yuv411_to_rgb", |b| b.iter(|| mcu.to_rgb(&sof)));

    let mcu = Mcu {
        blocks: vec![Block([0; 64]); 3],
    };
    let sof = StartOfFrameInfo {
        precision: 8,
        height: 1080,
        width: 1920,
        component_infos: [s1, s1, s1],
        max_horizontal_sampling: 1,
        max_vertical_sampling: 1,
    };
    c.bench_function("yuv444_to_rgb", |b| b.iter(|| mcu.to_rgb(&sof)));
}
