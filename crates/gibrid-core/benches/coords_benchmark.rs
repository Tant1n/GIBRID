use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gibrid_core::{block_to_chunk, block_to_local, chunk_to_region};

fn block_to_chunk_bench(c: &mut Criterion) {
    c.bench_function("block_to_chunk", |b| {
        b.iter(|| {
            let result = black_box(block_to_chunk(black_box(12345)));
            assert_eq!(result, 771);
        })
    });
}

fn block_to_local_bench(c: &mut Criterion) {
    c.bench_function("block_to_local", |b| {
        b.iter(|| {
            let result = black_box(block_to_local(black_box(12345)));
            assert_eq!(result, 9);
        })
    });
}

fn chunk_to_region_bench(c: &mut Criterion) {
    c.bench_function("chunk_to_region", |b| {
        b.iter(|| {
            let result = black_box(chunk_to_region(black_box(1000)));
            assert_eq!(result, 31);
        })
    });
}

criterion_group!(coords_benches, block_to_chunk_bench, block_to_local_bench, chunk_to_region_bench);
criterion_main!(coords_benches);
