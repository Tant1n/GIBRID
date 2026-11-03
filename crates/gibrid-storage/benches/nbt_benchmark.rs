// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gibrid_storage::nbt::{parse_root, Limits};

fn nbt_parse_small(c: &mut Criterion) {
    let data: Vec<u8> = vec![
        0x0A, 0, 0, // TAG_COMPOUND, empty name
        0x03, 0, 1, b'a', 0, 0, 0, 42, // int "a" = 42
        0x08, 0, 5, b'h', b'e', b'l', b'l', b'o', // string "hello" = 5
        0x00, // TAG_END
    ];

    c.bench_function("nbt_parse_small", |b| {
        b.iter(|| {
            let result = parse_root(black_box(&data), &Limits::default());
            assert!(result.is_ok());
        })
    });
}

fn nbt_parse_nested(c: &mut Criterion) {
    // Глубоко вложенный NBT (10 уровней)
    let mut data = Vec::new();
    for _ in 0..10 {
        data.push(0x0A); // TAG_COMPOUND
        data.push(0);
        data.push(0);
    }
    data.push(0x00); // TAG_END
    for _ in 0..10 {
        data.push(0x00); // TAG_END
    }

    c.bench_function("nbt_parse_nested", |b| {
        b.iter(|| {
            let result = parse_root(black_box(&data), &Limits::default());
            assert!(result.is_ok());
        })
    });
}

fn nbt_parse_large_array(c: &mut Criterion) {
    // Массив из 1000 int
    let mut data = Vec::new();
    data.push(0x0A); // TAG_COMPOUND
    data.push(0);
    data.push(0); // empty name
    data.push(0x0B); // TAG_INT_ARRAY
    data.push(0);
    data.push(1);
    data.push(b'a'); // name "a"
    data.push(0);
    data.push(0);
    data.push(0);
    data.push(1000u32.to_be_bytes()[0]);
    data.push(1000u32.to_be_bytes()[1]);
    data.push(1000u32.to_be_bytes()[2]);
    data.push(1000u32.to_be_bytes()[3]); // length = 1000
    for i in 0..1000 {
        data.push((i >> 24) as u8);
        data.push((i >> 16) as u8);
        data.push((i >> 8) as u8);
        data.push(i as u8);
    }
    data.push(0x00); // TAG_END

    c.bench_function("nbt_parse_large_array", |b| {
        b.iter(|| {
            let result = parse_root(black_box(&data), &Limits::default());
            assert!(result.is_ok());
        })
    });
}

criterion_group!(nbt_benches, nbt_parse_small, nbt_parse_nested, nbt_parse_large_array);
criterion_main!(nbt_benches);

