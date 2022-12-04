use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use jpeg_labs::{
    mcu::{BitReader, Block, Mcu},
    quantization_table::QuantizationTable,
    start_of_frame_0::{ComponentInfo, StartOfFrameInfo},
};
use smallvec::smallvec;
use std::io::BufReader;

criterion_group!(benches, block, mcu, bitreader);
criterion_main!(benches);

fn block(c: &mut Criterion) {
    let block = Block([0; 64]);
    c.bench_function("idct", |b| b.iter(|| block.idct()));
    c.bench_function("zigzag", |b| b.iter(|| block.zigzag()));
    c.bench_function("dequantize", |b| b.iter(|| block.dequantize(&[1; 64])));
    c.bench_function("upsample", |b| b.iter(|| block.upsample_2x2(0, 0)));
}

fn mcu(c: &mut Criterion) {
    let mut mcu = Mcu {
        blocks: smallvec![Block([0; 64]); 6],
    };
    let qts = [
        QuantizationTable {
            id: 0,
            values: [1; 64],
        },
        QuantizationTable {
            id: 1,
            values: [1; 64],
        },
    ];
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
    c.bench_function("yuv420_itrans", |b| b.iter(|| mcu.itrans(&sof, &qts)));
    c.bench_function("yuv420_to_rgb", |b| b.iter(|| mcu.to_rgb(&sof)));

    let mut mcu = Mcu {
        blocks: smallvec![Block([0; 64]); 3],
    };
    let sof = StartOfFrameInfo {
        precision: 8,
        height: 1080,
        width: 1920,
        component_infos: [s1, s1, s1],
        max_horizontal_sampling: 1,
        max_vertical_sampling: 1,
    };
    c.bench_function("yuv444_itrans", |b| b.iter(|| mcu.itrans(&sof, &qts)));
    c.bench_function("yuv444_to_rgb", |b| b.iter(|| mcu.to_rgb(&sof)));
}

fn bitreader(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_value");

    for size in [1, 2, 4, 8, 16] {
        let mut reader = BitReader::new(BufReader::new(std::io::repeat(0x00)));
        group.throughput(Throughput::Bytes(size));
        group.bench_function(size.to_string(), |b| {
            b.iter(|| reader.read_value(size as u8))
        });
    }
}
