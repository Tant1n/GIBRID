// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gibrid_mesh::{build_section_mesh, SectionMeshInput};
use gibrid_core::ResourceId;

fn mesh_single_block(c: &mut Criterion) {
    let palette: Vec<ResourceId> = vec![
        ResourceId::minecraft("air").unwrap(),
        ResourceId::minecraft("stone").unwrap(),
    ];
    let mut packed: Vec<u64> = vec![0; 64];
    let pos = ((8 * 16 + 8) * 16 + 8) as usize;
    let bit_start = pos * 1;
    let word = bit_start / 64;
    let offset = bit_start % 64;
    packed[word] |= 1u64 << offset;

    let input = SectionMeshInput {
        section_x: 0,
        section_y: 0,
        section_z: 0,
        palette: &palette,
        packed_data: &packed,
        bits_per_block: 1,
        is_full_cube: &|id: &ResourceId| id.path() == "stone",
        get_block: &|_, _, _| None,
    };

    c.bench_function("mesh_single_block", |b| {
        b.iter(|| {
            let mesh = build_section_mesh(black_box(&input));
            assert!(!mesh.is_empty());
        })
    });
}

fn mesh_full_chunk(c: &mut Criterion) {
    // Вся секция заполнена камнем (все 4096 блоков)
    let palette: Vec<ResourceId> = vec![
        ResourceId::minecraft("air").unwrap(),
        ResourceId::minecraft("stone").unwrap(),
    ];
    let mut packed: Vec<u64> = vec![0; 64];
    // Все блоки - камень (индекс 1)
    for i in 0..64 {
        packed[i] = u64::MAX;
    }

    let input = SectionMeshInput {
        section_x: 0,
        section_y: 0,
        section_z: 0,
        palette: &palette,
        packed_data: &packed,
        bits_per_block: 1,
        is_full_cube: &|id: &ResourceId| id.path() == "stone",
        get_block: &|_, _, _| None,
    };

    c.bench_function("mesh_full_chunk", |b| {
        b.iter(|| {
            let mesh = build_section_mesh(black_box(&input));
            assert_eq!(mesh.index_count, 0); // Все соседи тоже камень, грани скрыты
        })
    });
}

criterion_group!(mesh_benches, mesh_single_block, mesh_full_chunk);
criterion_main!(mesh_benches);

