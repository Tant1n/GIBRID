// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Сущности и базовая физика AABB.
//! Статус: скелет полноценного ядра. Игрок, мобы, предметы — все сущности с позицией,
//! скоростью, коллизионным боксом и интеграцией движения.

use glam::Vec3;
use std::collections::HashMap;

/// Тип сущности.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Player,
    Mob,
    Item,
    Projectile,
}

/// Коллизионный бокс.
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x
            && self.min.y < other.max.y && self.max.y > other.min.y
            && self.min.z < other.max.z && self.max.z > other.min.z
    }
}

/// Сущность в мире.
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u64,
    pub kind: EntityKind,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub bb: AABB,
}

impl Entity {
    pub fn new(id: u64, kind: EntityKind, pos: Vec3) -> Self {
        Self {
            id,
            kind,
            position: pos,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            bb: AABB::new(
                pos + Vec3::new(-0.3, 0.0, -0.3),
                pos + Vec3::new(0.3, 1.8, 0.3),
            ),
        }
    }

    pub fn update_bb(&mut self) {
        self.bb = AABB::new(
            self.position + Vec3::new(-0.3, 0.0, -0.3),
            self.position + Vec3::new(0.3, 1.8, 0.3),
        );
    }
}

/// Контейнер сущностей мира.
pub struct EntityWorld {
    pub entities: HashMap<u64, Entity>,
    next_id: u64,
}

impl Default for EntityWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityWorld {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn spawn(&mut self, kind: EntityKind, pos: Vec3) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.insert(id, Entity::new(id, kind, pos));
        id
    }

    pub fn get(&self, id: u64) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }
}

/// Базовая физика: гравитация и интеграция движения.
pub struct Physics;

impl Physics {
    pub const GRAVITY: f32 = 20.0;

    pub fn apply_gravity(e: &mut Entity, dt: f32) {
        e.velocity.y -= Self::GRAVITY * dt;
    }

    pub fn integrate(e: &mut Entity, dt: f32) {
        e.position += e.velocity * dt;
        e.update_bb();
    }

    pub fn resolve_aabb(e: &mut Entity, other: &AABB) -> bool {
        if !e.bb.intersects(other) {
            return false;
        }
        // Выталкивание по наименьшей оси (упрощённо)
        let overlap_x = (e.bb.max.x - other.min.x).min(other.max.x - e.bb.min.x);
        let overlap_y = (e.bb.max.y - other.min.y).min(other.max.y - e.bb.min.y);
        let overlap_z = (e.bb.max.z - other.min.z).min(other.max.z - e.bb.min.z);

        if overlap_y <= overlap_x && overlap_y <= overlap_z {
            if e.position.y < other.min.y {
                e.position.y -= overlap_y;
                e.velocity.y = 0.0;
                e.on_ground = true;
            } else {
                e.position.y += overlap_y;
                e.velocity.y = 0.0;
            }
        } else if overlap_x <= overlap_z {
            if e.position.x < other.min.x {
                e.position.x -= overlap_x;
            } else {
                e.position.x += overlap_x;
            }
            e.velocity.x = 0.0;
        } else {
            if e.position.z < other.min.z {
                e.position.z -= overlap_z;
            } else {
                e.position.z += overlap_z;
            }
            e.velocity.z = 0.0;
        }
        e.update_bb();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_spawn_and_move() {
        let mut world = EntityWorld::new();
        let id = world.spawn(EntityKind::Player, Vec3::new(0.0, 64.0, 0.0));
        let e = world.get(id).unwrap();
        assert_eq!(e.position, Vec3::new(0.0, 64.0, 0.0));

        Physics::apply_gravity(&mut world.entities.get_mut(&id).unwrap(), 0.05);
        let e = world.get(id).unwrap();
        assert!(e.velocity.y < 0.0);
    }

    #[test]
    fn aabb_intersects() {
        let a = AABB::new(Vec3::ZERO, Vec3::splat(1.0));
        let b = AABB::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(1.5, 1.5, 1.5));
        assert!(a.intersects(&b));

        let c = AABB::new(Vec3::new(2.0, 2.0, 2.0), Vec3::new(3.0, 3.0, 3.0));
        assert!(!a.intersects(&c));
    }
}
