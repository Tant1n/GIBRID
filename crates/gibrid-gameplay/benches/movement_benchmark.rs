// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gibrid_core::ResourceId;
use gibrid_gameplay::{
    input::InputHandler,
    movement::apply_movement,
    Player,
    WorldContext,
};
use gibrid_mesh::mesh_queue::MeshQueue;

fn player_movement_bench(c: &mut Criterion) {
    let mut player = Player::new(0.0, 64.0, 0.0);
    let mut input = InputHandler::new();
    let mesh_queue = MeshQueue::new(16, 16);
    let mut world = WorldContext::new(mesh_queue);

    // Ставим несколько блоков под ногами
    let stone = ResourceId::minecraft("stone").unwrap();
    for x in -2..=2 {
        for z in -2..=2 {
            world.set_block(x, 62, z, Some(stone.clone()));
        }
    }

    c.bench_function("player_movement", |b| {
        b.iter(|| {
            apply_movement(black_box(&mut player), black_box(&input), black_box(&world), 0.016);
        })
    });
}

criterion_group!(movement_benches, player_movement_bench);
criterion_main!(movement_benches);

