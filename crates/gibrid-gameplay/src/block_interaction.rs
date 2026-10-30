//! Взаимодействие с блоками: raycasting, ломание и установка блоков.
//! Реализует простой DDA (цифровой дифференциальный анализатор) для прохода по чанкам.

use crate::Player;
use crate::WorldContext;
use gibrid_core::ResourceId;
use glam::Vec3;

/// Результат луч-время пересечения с блоком.
#[derive(Debug, Clone, Copy)]
pub struct BlockHit {
    pub position: glam::Vec3,   // позиция блока в мире
    pub normal: glam::Vec3,     // нормаль грани, по которой попали
    pub distance: f32,          // расстояние от игрока до блока
}

/// Луч от игрока в направлении взгляда.
pub fn raycast(player: &Player, world: &crate::WorldContext, max_distance: f32) -> Option<BlockHit> {
    let origin = player.position + glam::Vec3::new(0.0, 1.62, 0.0); // глаза игрока
    let direction = player.forward().normalize();

    let mut pos = origin;
    let dir = direction;

    // Шаг для DDA
    let step = 0.1; // шаг в метрах
    let max_steps = (max_distance / step) as i32;

    for _ in 0..max_steps {
        pos += dir * step;

        let bx = pos.x.floor() as i32;
        let by = pos.y.floor() as i32;
        let bz = pos.z.floor() as i32;

        if let Some(block) = world.get_block(bx, by, bz) {
            if block.path() != "air" {
                // Нашли блок — определяем нормаль грани
                let normal = calculate_hit_normal(pos, dir, bx, by, bz);
                return Some(BlockHit {
                    position: glam::Vec3::new(bx as f32, by as f32, bz as f32),
                    normal,
                    distance: (pos - origin).length(),
                });
            }
        }
    }

    None
}

/// Вычислить нормаль грани, по которой ударились, основываясь на позиции внутри блока.
fn calculate_hit_normal(hit_pos: Vec3, _dir: Vec3, bx: i32, by: i32, bz: i32) -> Vec3 {
    let local = glam::Vec3::new(
        hit_pos.x - bx as f32,
        hit_pos.y - by as f32,
        hit_pos.z - bz as f32,
    );

    // Находим грань,closest к которой мы подошли
    let dist_to_min_x = local.x;
    let dist_to_max_x = 1.0 - local.x;
    let dist_to_min_y = local.y;
    let dist_to_max_y = 1.0 - local.y;
    let dist_to_min_z = local.z;
    let dist_to_max_z = 1.0 - local.z;

    let min_dist = dist_to_min_x
        .min(dist_to_max_x)
        .min(dist_to_min_y)
        .min(dist_to_max_y)
        .min(dist_to_min_z)
        .min(dist_to_max_z);

    if min_dist == dist_to_min_x {
        glam::Vec3::new(-1.0, 0.0, 0.0)
    } else if min_dist == dist_to_max_x {
        glam::Vec3::new(1.0, 0.0, 0.0)
    } else if min_dist == dist_to_min_y {
        glam::Vec3::new(0.0, -1.0, 0.0)
    } else if min_dist == dist_to_max_y {
        glam::Vec3::new(0.0, 1.0, 0.0)
    } else if min_dist == dist_to_min_z {
        glam::Vec3::new(0.0, 0.0, -1.0)
    } else {
        glam::Vec3::new(0.0, 0.0, 1.0)
    }
}

/// Ломать блок под прицелом.
pub fn break_block(world: &mut WorldContext, hit: &BlockHit) {
    let x = hit.position.x.round() as i32;
    let y = hit.position.y.round() as i32;
    let z = hit.position.z.round() as i32;
    world.set_block(x, y, z, None);
}

/// Поставить блок рядом с блоком под прицелом (в направлении нормали).
pub fn place_block(world: &mut WorldContext, hit: &BlockHit, block: ResourceId) {
    let x = (hit.position.x + hit.normal.x).round() as i32;
    let y = (hit.position.y + hit.normal.y).round() as i32;
    let z = (hit.position.z + hit.normal.z).round() as i32;
    world.set_block(x, y, z, Some(block));
}
