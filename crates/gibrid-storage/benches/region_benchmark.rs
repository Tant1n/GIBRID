// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gibrid_storage::region::{read_chunk, RegionHeader};
use gibrid_storage::nbt::{parse_root, Limits};

fn region_header_parse(c: &mut Criterion) {
    let header_bytes = vec![0u8; 8192]; // пустой заголовок региона

    c.bench_function("region_header_parse", |b| {
        b.iter(|| {
            let result = RegionHeader::parse(black_box(&header_bytes));
            assert!(result.is_ok());
        })
    });
}

fn chunk_read(c: &mut Criterion) {
    // Создаем минимальный валидный регион с одним чанком
    let mut data = vec![0u8; 8192 + 4096]; // заголовок + 1 сектор
    // location index 0: sector 2, count 1
    data[2] = 2;
    data[3] = 1;
    let start = 2 * 4096;
    // length = 5 (1 compression + 4 data)
    data[start..start + 4].copy_from_slice(&5u32.to_be_bytes());
    data[start + 4] = 3; // Uncompressed
    data[start + 5..start + 9].copy_from_slice(&[1, 2, 3, 4]);

    let header = RegionHeader::parse(&data).unwrap();
    let loc = header.location(0).unwrap();

    c.bench_function("chunk_read", |b| {
        b.iter(|| {
            let result = read_chunk(black_box(&data), black_box(loc));
            assert!(result.is_ok());
        })
    });
}

fn nbt_parse(c: &mut Criterion) {
    let nbt_bytes: Vec<u8> = vec![
        0x0A, 0, 0, // TAG_COMPOUND, empty name
        0x03, 0, 1, b'a', 0, 0, 0, 42, // int "a" = 42
        0x08, 0, 5, b'h', b'e', b'l', b'l', b'o', // string "hello"
        0x00, // TAG_END
    ];

    c.bench_function("nbt_parse", |b| {
        b.iter(|| {
            let result = parse_root(black_box(&nbt_bytes), &Limits::default());
            assert!(result.is_ok());
        })
    });
}

criterion_group!(region_benches, region_header_parse, chunk_read, nbt_parse);
criterion_main!(region_benches);

