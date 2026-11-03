// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.

// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.//! GIBRID Gameplay — минимальный игровой цикл (Phase 5).
//!
//! Включает: игрока, ввод (клавиатура/мышь), движение с коллизиями,
//! луч для ломания/установки блоков, связь с очередью мешинга.
//! Статус: IMPLEMENTED (базовый функционал без текстур и звука).

#![forbid(unsafe_code)]

pub mod crafting;
pub mod input;
pub mod inventory;
pub mod movement;
pub mod block_interaction;
pub mod mobs;

use gibrid_core::ResourceId;
use gibrid_mesh::{FaceTextureMap, MeshJob, MeshQueue, MeshRevision, TextureMap};
use std::collections::HashMap;
use std::sync::Arc;

/// Состояние игрока: позиция, углы, скорость, инвентарь.
#[derive(Debug, Clone)]
pub struct Player {
    pub position: glam::Vec3,
    pub yaw: f32,   // вращение вокруг Y (влево/вправо)
    pub pitch: f32, // вращение вокруг X (вверх/вниз)
    pub velocity: glam::Vec3,
    pub on_ground: bool,
    pub inventory: inventory::Inventory,
}

impl Player {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: glam::Vec3::new(x, y, z),
            yaw: 0.0,
            pitch: 0.0,
            velocity: glam::Vec3::ZERO,
            on_ground: false,
            inventory: inventory::Inventory::new(
                inventory::ContainerDefinition::player_inventory(),
            ),
        }
    }

    /// Переместить игрока на delta.
    pub fn move_by(&mut self, delta: glam::Vec3) {
        self.position += delta;
    }

    /// Повернуть игрока.
    pub fn rotate(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw += dyaw;
        self.pitch += dpitch;
        self.pitch = self.pitch.clamp(-89.0, 89.0);
    }

    /// Направление взгляда игрока (вектор вперёд).
    pub fn forward(&self) -> glam::Vec3 {
        let yaw = self.yaw.to_radians();
        let pitch = self.pitch.to_radians();
        glam::Vec3::new(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
        .normalize()
    }

    /// Направление вправо.
    pub fn right(&self) -> glam::Vec3 {
        let yaw = self.yaw.to_radians();
        glam::Vec3::new(yaw.sin(), 0.0, -yaw.cos()).normalize()
    }

    /// Направление вперёд в плоскости XZ (без учета наклона).
    pub fn forward_flat(&self) -> glam::Vec3 {
        let yaw = self.yaw.to_radians();
        glam::Vec3::new(yaw.cos(), 0.0, yaw.sin()).normalize()
    }

    /// Направление вправо в плоскости XZ.
    pub fn right_flat(&self) -> glam::Vec3 {
        let yaw = self.yaw.to_radians();
        glam::Vec3::new(yaw.sin(), 0.0, -yaw.cos()).normalize()
    }
}

/// Контекст мира для gameplay: доступ к чанкам и меш-очереди.
pub struct WorldContext {
    /// chunk key -> блок-состояние (пока простая карта)
    pub chunks: HashMap<(i32, i32, i32), ChunkData>,
    pub mesh_queue: MeshQueue,
    pub next_revision: MeshRevision,
    /// Карта текстур: индекс в атласе для каждого блока.
    pub texture_map: Arc<TextureMap>,
    /// Карта текстур по граням: (блок, грань) -> индекс в атласе.
    pub face_texture_map: Arc<FaceTextureMap>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub tile_size: u32,
}

/// Данные чанка (упрощённые: палитра + packed данные).
pub struct ChunkData {
    pub palette: Vec<ResourceId>,
    pub packed_data: Vec<u64>,
    pub bits_per_block: u32,
}

impl WorldContext {
    pub fn new(
        mesh_queue: MeshQueue,
        texture_map: Arc<TextureMap>,
        face_texture_map: Arc<FaceTextureMap>,
        atlas_width: u32,
        atlas_height: u32,
        tile_size: u32,
    ) -> Self {
        Self {
            chunks: HashMap::new(),
            mesh_queue,
            next_revision: MeshRevision(1),
            texture_map,
            face_texture_map,
            atlas_width,
            atlas_height,
            tile_size,
        }
    }

    /// Получить блок по мировым координатам.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<ResourceId> {
        let chunk_x = (x as f32 / 16.0).floor() as i32;
        let chunk_z = (z as f32 / 16.0).floor() as i32;
        let key = (chunk_x, 0, chunk_z); // упрощённо: только одна секция по Y

        let chunk_data = self.chunks.get(&key)?;
        let local_x = x.rem_euclid(16);
        let local_y = y.rem_euclid(16);
        let local_z = z.rem_euclid(16);

        let palette_idx = gibrid_mesh::get_palette_index(
            &chunk_data.packed_data,
            chunk_data.bits_per_block,
            local_x,
            local_y,
            local_z,
        )?;
        chunk_data.palette.get(palette_idx).cloned()
    }

    /// Получить готовый меш из очереди, если есть.
    pub fn try_recv_mesh(&mut self) -> Option<gibrid_mesh::MeshResult> {
        self.mesh_queue.try_recv()
    }

    /// Установить блок по мировым координатам и отправить на перемешивание.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Option<ResourceId>) {
        let chunk_x = (x as f32 / 16.0).floor() as i32;
        let chunk_z = (z as f32 / 16.0).floor() as i32;
        let key = (chunk_x, 0, chunk_z);

        let local_x = x.rem_euclid(16);
        let local_y = y.rem_euclid(16);
        let local_z = z.rem_euclid(16);

        // Получить или создать чанк
        let chunk_data = self.chunks.entry(key).or_insert(ChunkData {
            palette: vec![ResourceId::minecraft("air").unwrap()],
            packed_data: vec![0; 64],
            bits_per_block: 1,
        });

        // Найти или добавить блок в палитру
        let block_idx = if let Some(block) = block {
            match chunk_data.palette.iter().position(|b| *b == block) {
                Some(idx) => idx,
                None => {
                    chunk_data.palette.push(block);
                    chunk_data.palette.len() - 1
                }
            }
        } else {
            0 // air
        };

        // Установить бит в packed_data
        let pos = ((local_y * 16 + local_z) * 16 + local_x) as usize;
        let bit_start = pos * chunk_data.bits_per_block as usize;
        let word = bit_start / 64;
        let offset = bit_start % 64;

        // Расширить packed_data если нужно
        while chunk_data.packed_data.len() <= word {
            chunk_data.packed_data.push(0);
        }

        if block_idx > 0 {
            chunk_data.packed_data[word] |= (block_idx as u64) << offset;
        } else {
            chunk_data.packed_data[word] &= !(1u64 << offset);
        }

        // Создать задачу на мешинг
        let revision = self.next_revision.0;
        self.next_revision.0 += 1;

        let job = MeshJob {
            section_x: chunk_x,
            section_y: 0,
            section_z: chunk_z,
            revision: MeshRevision(revision),
            palette: chunk_data.palette.clone(),
            packed_data: chunk_data.packed_data.clone(),
            bits_per_block: chunk_data.bits_per_block,
            texture_map: self.texture_map.clone(),
            face_texture_map: self.face_texture_map.clone(),
            atlas_width: self.atlas_width,
            atlas_height: self.atlas_height,
            tile_size: self.tile_size,
        };

        let _ = self.mesh_queue.submit(job);
    }
}