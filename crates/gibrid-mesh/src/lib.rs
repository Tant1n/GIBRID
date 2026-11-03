// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.

// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.//! Чанковые меши: построение вершинных данных из секций чанка.
//!
//! Статус: Phase 4, части 1 и 2 — меши полных кубов с face culling,
//! ревизии мешей, очереди задач, инвалидация соседей.
//! Не сделано: UV анимация, прозрачность, неполные кубы, освещение.

#![forbid(unsafe_code)]

pub mod mesh_queue;

pub use mesh_queue::MeshQueue;

use bytemuck_derive::{Pod as BytemuckPod, Zeroable as BytemuckZeroable};
use gibrid_core::ResourceId;
use std::collections::HashMap;
use std::sync::Arc;

/// Вершина чанкового меша.
#[repr(C)]
#[derive(Clone, Copy, BytemuckPod, BytemuckZeroable, Debug)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal:   [f32; 3],
    pub uv:       [f32; 2],      // Локальные UV грани (0..1)
    pub color:    [u8; 4],
    /// Смещение текстуры в атласе (в пикселях).
    pub uv_offset: [f32; 2],
    /// Размер текстуры в пикселях (ширина, высота).
    pub uv_scale: [f32; 2],
}

/// Готовый меш одной секции чанка.
#[derive(Debug, Clone)]
pub struct ChunkMesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    pub index_count: u32,
    pub section_x: i32,
    pub section_y: i32,
    pub section_z: i32,
}

impl ChunkMesh {
    pub fn empty(section_x: i32, section_y: i32, section_z: i32) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            index_count: 0,
            section_x,
            section_y,
            section_z,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.index_count == 0
    }
}

/// Направление грани.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Down,   // -Y
    Up,     // +Y
    North,  // -Z
    South,  // +Z
    West,   // -X
    East,   // +X
}

impl Face {
    pub fn all() -> [Face; 6] {
        [
            Face::Down,
            Face::Up,
            Face::North,
            Face::South,
            Face::West,
            Face::East,
        ]
    }

    /// Смещение соседа в направлении грани.
    pub fn neighbor_offset(self) -> (i32, i32, i32) {
        match self {
            Face::Down => (0, -1, 0),
            Face::Up => (0, 1, 0),
            Face::North => (0, 0, -1),
            Face::South => (0, 0, 1),
            Face::West => (-1, 0, 0),
            Face::East => (1, 0, 0),
        }
    }

    /// Нормаль грани.
    pub fn normal(self) -> [f32; 3] {
        match self {
            Face::Down => [0.0, -1.0, 0.0],
            Face::Up => [0.0, 1.0, 0.0],
            Face::North => [0.0, 0.0, -1.0],
            Face::South => [0.0, 0.0, 1.0],
            Face::West => [-1.0, 0.0, 0.0],
            Face::East => [1.0, 0.0, 0.0],
        }
    }
}

/// Функция проверки: является ли блок полным кубом (непрозрачным, блокирующим свет и вид).
pub type IsFullCubeFn = dyn Fn(&ResourceId) -> bool;

/// Функция получения блока по локальным координатам секции (0..16).
/// Возвращает None, если блок не существует (воздух).
pub type GetBlockFn = dyn Fn(i32, i32, i32) -> Option<ResourceId>;

/// Карта текстур: индекс в атласе для каждого блока.
pub type TextureMap = HashMap<ResourceId, u32>;

/// Карта текстур по граням: (блок, грань) -> индекс в атласе.
/// Если нет записи для конкретной грани, используется texture_map[блок].
pub type FaceTextureMap = HashMap<(ResourceId, Face), u32>;

/// Входные данные для построения меша секции.
pub struct SectionMeshInput<'a> {
    /// Позиция секции в чанковых координатах.
    pub section_x: i32,
    pub section_y: i32,
    pub section_z: i32,
    /// Палитра блок-состояний (индекс -> ResourceId).
    pub palette: &'a [ResourceId],
    /// Packed данные секции (bits_per_block бит на блок, 4096 блоков).
    pub packed_data: &'a [u64],
    /// Бит на блок.
    pub bits_per_block: u32,
    /// Проверка: является ли блок полным кубом.
    pub is_full_cube: &'a IsFullCubeFn,
    /// Получение блока по локальным координатам секции (0..16).
    pub get_block: &'a GetBlockFn,
    /// Карта текстур: индекс в атласе для каждого блока.
    pub texture_map: &'a Arc<TextureMap>,
    /// Карта текстур по граням: (блок, грань) -> индекс в атласе.
    pub face_texture_map: &'a Arc<FaceTextureMap>,
    /// Размер атласа в пикселях.
    pub atlas_width: u32,
    pub atlas_height: u32,
    /// Размер одной текстурной плитки в пикселях (обычно 16x16).
    pub tile_size: u32,
    /// Получение яркости блока (0..15) по локальным координатам.
    pub get_brightness: &'a dyn Fn(i32, i32, i32) -> u8,
}

/// Получить индекс блок-состояния из packed данных по локальным координатам (x, y, z).
pub fn get_palette_index(packed: &[u64], bits_per_block: u32, x: i32, y: i32, z: i32) -> Option<usize> {
    if x < 0 || x >= 16 || y < 0 || y >= 16 || z < 0 || z >= 16 {
        return None;
    }
    if bits_per_block == 0 {
        // Вся секция один блок — индекс 0
        return Some(0);
    }
    let pos = ((y * 16 + z) * 16 + x) as usize;
    let bit_start = pos * bits_per_block as usize;
    let mut value: u64 = 0;
    for i in 0..bits_per_block as usize {
        let bit = bit_start + i;
        let word = bit / 64;
        let offset = bit % 64;
        if word >= packed.len() {
            return None;
        }
        if (packed[word] >> offset) & 1 == 1 {
            value |= 1u64 << i;
        }
    }
    Some(value as usize)
}

/// Построить меш секции с face culling.
pub fn build_section_mesh(input: &SectionMeshInput) -> ChunkMesh {
    let mut vertices = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                let palette_idx = match get_palette_index(
                    input.packed_data,
                    input.bits_per_block,
                    x,
                    y,
                    z,
                ) {
                    Some(idx) => idx,
                    None => continue,
                };
                if palette_idx >= input.palette.len() {
                    continue;
                }
                let block_id = &input.palette[palette_idx];
                if !(input.is_full_cube)(block_id) {
                    continue;
                }

                // Мировые координаты блока
                let wx = input.section_x * 16 + x;
                let wy = input.section_y * 16 + y;
                let wz = input.section_z * 16 + z;

                // Яркость блока (0..15)
                let brightness = (input.get_brightness)(x, y, z);

                for face in Face::all() {
                    let (nx, ny, nz) = face.neighbor_offset();
                    let neighbor_id = (input.get_block)(wx + nx, wy + ny, wz + nz);
                    let neighbor_is_full = neighbor_id
                        .map(|id| (input.is_full_cube)(&id))
                        .unwrap_or(false);
                    if neighbor_is_full {
                        continue; // Грань невидима — соседи блокируют вид
                    }

                    add_face(
                        &mut vertices,
                        &mut indices,
                        input.section_x,
                        input.section_y,
                        input.section_z,
                        x as f32,
                        y as f32,
                        z as f32,
                        face,
                        block_id,
                        input.texture_map,
                        input.face_texture_map,
                        input.atlas_width,
                        input.atlas_height,
                        input.tile_size,
                        brightness,
                    );
                }
            }
        }
    }

    let index_count = indices.len() as u32;
    ChunkMesh {
        vertices,
        indices,
        index_count,
        section_x: input.section_x,
        section_y: input.section_y,
        section_z: input.section_z,
    }
}

/// Добавить одну грань (два треугольника) к мешу.
fn add_face(
    vertices: &mut Vec<MeshVertex>,
    indices: &mut Vec<u32>,
    sx: i32,
    sy: i32,
    sz: i32,
    x: f32,
    y: f32,
    z: f32,
    face: Face,
    block_id: &ResourceId,
    texture_map: &Arc<TextureMap>,
    face_texture_map: &FaceTextureMap,
    atlas_width: u32,
    atlas_height: u32,
    tile_size: u32,
    brightness: u8,
) {
    let normal = face.normal();

    // Индекс текстуры в атласе: сначала ищем по (блок, грань), затем fallback на блок
    let texture_index = face_texture_map
        .get(&(block_id.clone(), face))
        .copied()
        .or_else(|| texture_map.get(block_id).copied())
        .unwrap_or(0);

    // Вычисляем позицию текстуры в атласе (в пикселях)
    let cols = atlas_width / tile_size;
    let _rows = atlas_height / tile_size;
    let col = texture_index % cols;
    let row = texture_index / cols;
    let tex_x = col as f32 * tile_size as f32;
    let tex_y = row as f32 * tile_size as f32;

    // UV-развёртка грани в локальных координатах (0..1)
    // Порядок вершин: CCW при взгляде СНАРУЖИ (для cull_mode = Back)
    let (corners, uv_map): ([(f32, f32, f32); 4], [(f32, f32); 4]) = match face {
        // Down (-Y): looking from below, CCW = (0,0,0)→(0,0,1)→(1,0,1)→(1,0,0)
        Face::Down => (
            [(0.0, 0.0, 0.0), (0.0, 0.0, 1.0), (1.0, 0.0, 1.0), (1.0, 0.0, 0.0)],
            [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
        ),
        // Up (+Y): looking from above, CCW = (0,1,0)→(0,1,1)→(1,1,1)→(1,1,0)
        Face::Up => (
            [(0.0, 1.0, 0.0), (0.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 0.0)],
            [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
        ),
        // North (-Z): looking from -Z, CCW = (0,0,0)→(1,0,0)→(1,1,0)→(0,1,0)
        Face::North => (
            [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 1.0, 0.0)],
            [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
        ),
        // South (+Z): looking from +Z, CCW = (1,0,1)→(1,1,1)→(0,1,1)→(0,0,1)
        Face::South => (
            [(1.0, 0.0, 1.0), (1.0, 1.0, 1.0), (0.0, 1.0, 1.0), (0.0, 0.0, 1.0)],
            [(1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)],
        ),
        // West (-X): looking from -X, CCW = (0,0,0)→(0,1,0)→(0,1,1)→(0,0,1)
        Face::West => (
            [(0.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 1.0, 1.0), (0.0, 0.0, 1.0)],
            [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
        ),
        // East (+X): looking from +X, CCW = (1,0,1)→(1,1,1)→(1,1,0)→(1,0,0)
        Face::East => (
            [(1.0, 0.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0, 0.0), (1.0, 0.0, 0.0)],
            [(1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)],
        ),
    };

    let base_index = vertices.len() as u32;
    // Яркость 0..15 -> 0..255
    let brightness = (brightness as f32 / 15.0 * 255.0).clamp(0.0, 255.0) as u8;
    let color = [brightness, brightness, brightness, 255];

    for i in 0..4 {
        let (cx, cy, cz) = corners[i];
        let (u, v) = uv_map[i];
        // Преобразуем UV из локальных (0..1) в глобальные координаты в атласе
        let atlas_u = tex_x + u * tile_size as f32;
        let atlas_v = tex_y + v * tile_size as f32;
        // Мировые координаты = section_offset + локальные координаты блока + смещение внутри грани
        vertices.push(MeshVertex {
            position: [(sx as f32 * 16.0) + x + cx, (sy as f32 * 16.0) + y + cy, (sz as f32 * 16.0) + z + cz],
            normal,
            uv: [u, v],
            color,
            uv_offset: [atlas_u, atlas_v],
            uv_scale: [tile_size as f32, tile_size as f32],
        });
    }

    // Два треугольника (по часовой стрелке при взгляде снаружи)
    indices.push(base_index);
    indices.push(base_index + 1);
    indices.push(base_index + 2);
    indices.push(base_index);
    indices.push(base_index + 2);
    indices.push(base_index + 3);
}

// ============================================================================
// Phase 4, часть 2 — Очереди мешинга, ревизии и инвалидация соседей
// ============================================================================

/// Монотонно растущая ревизия секции чанка.
/// Инкрементируется при каждом изменении данных секции.
/// Используется для отбрасывания устаревших результатов мешинга.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MeshRevision(pub u64);

impl MeshRevision {
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

/// Задача мешинга: снимок данных секции + ревизия.
pub struct MeshJob {
    pub section_x: i32,
    pub section_y: i32,
    pub section_z: i32,
    pub revision: MeshRevision,
    pub palette: Vec<ResourceId>,
    pub packed_data: Vec<u64>,
    pub bits_per_block: u32,
    pub texture_map: Arc<TextureMap>,
    pub face_texture_map: Arc<FaceTextureMap>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub tile_size: u32,
}

/// Результат мешинга: построенный меш + ревизия, на которой он построен.
pub struct MeshResult {
    pub mesh: ChunkMesh,
    pub revision: MeshRevision,
}

/// Проверка: устарел ли результат мешинга (ревизия не совпадает с текущей).
pub fn is_mesh_stale(result: &MeshResult, current_revision: MeshRevision) -> bool {
    result.revision != current_revision
}

/// Построить MeshJob из SectionMeshInput с заданной ревизией.
pub fn make_mesh_job(input: &SectionMeshInput, revision: MeshRevision) -> MeshJob {
    MeshJob {
        section_x: input.section_x,
        section_y: input.section_y,
        section_z: input.section_z,
        revision,
        palette: input.palette.to_vec(),
        packed_data: input.packed_data.to_vec(),
        bits_per_block: input.bits_per_block,
        texture_map: input.texture_map.clone(),
        face_texture_map: input.face_texture_map.clone(),
        atlas_width: input.atlas_width,
        atlas_height: input.atlas_height,
        tile_size: input.tile_size,
    }
}

/// Построить MeshResult из MeshJob, выполнив мешинг.
/// Все блоки считаются полными кубами (face culling без учёта соседей).
pub fn build_mesh_from_job(job: &MeshJob) -> MeshResult {
    let input = SectionMeshInput {
        section_x: job.section_x,
        section_y: job.section_y,
        section_z: job.section_z,
        palette: &job.palette,
        packed_data: &job.packed_data,
        bits_per_block: job.bits_per_block,
        is_full_cube: &|_| true,
        get_block: &|_, _, _| None, // Соседи неизвестны на этом этапе
        texture_map: &job.texture_map,
        face_texture_map: &job.face_texture_map,
        atlas_width: job.atlas_width,
        atlas_height: job.atlas_height,
        tile_size: job.tile_size,
        get_brightness: &|_, _, _| 15, // Яркость по умолчанию (максимальная)
    };
    let mesh = build_section_mesh(&input);
    MeshResult {
        mesh,
        revision: job.revision,
    }
}

/// Инвалидация соседей при изменении блока.
/// Возвращает уникальные координаты секций, которые могут потребовать перемешивания.
/// Если блок на границе чанка, добавляются соседние секции в соседних чанках.
pub fn neighbors_for_invalidation(
    section_x: i32,
    section_y: i32,
    section_z: i32,
    local_x: i32,
    local_y: i32,
    local_z: i32,
) -> Vec<(i32, i32, i32)> {
    use std::collections::HashSet;

    let mut sections: HashSet<(i32, i32, i32)> = HashSet::new();

    // Соседние секции внутри того же чанка (включая текущую)
    for dx in -1..=1i32 {
        for dy in -1..=1i32 {
            for dz in -1..=1i32 {
                let nx = local_x + dx;
                let ny = local_y + dy;
                let nz = local_z + dz;

                // Если сосед в пределах секции (0..16) — та же секция
                if nx >= 0 && nx < 16 && ny >= 0 && ny < 16 && nz >= 0 && nz < 16 {
                    continue; // та же секция, уже учтена
                }

                // Сосед за границей секции — пересчитываем координаты секции
                let mut sx = section_x;
                let mut sy = section_y;
                let mut sz = section_z;

                if nx < 0 {
                    sx -= 1;
                } else if nx >= 16 {
                    sx += 1;
                }
                if ny < 0 {
                    sy -= 1;
                } else if ny >= 16 {
                    sy += 1;
                }
                if nz < 0 {
                    sz -= 1;
                } else if nz >= 16 {
                    sz += 1;
                }

                sections.insert((sx, sy, sz));
            }
        }
    }

    // Добавляем саму секцию блока
    sections.insert((section_x, section_y, section_z));

    sections.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Только камень — полный куб.
    fn is_stone(id: &ResourceId) -> bool {
        id.path() == "stone"
    }

    #[test]
    fn empty_section_produces_empty_mesh() {
        let palette: Vec<ResourceId> = Vec::new();
        let packed: Vec<u64> = Vec::new();
        let mut texture_map: TextureMap = HashMap::new();
        texture_map.insert(ResourceId::minecraft("stone").unwrap(), 0);
        texture_map.insert(ResourceId::minecraft("air").unwrap(), 0);
        let face_texture_map: Arc<FaceTextureMap> = Arc::new(HashMap::new());
        let input = SectionMeshInput {
            section_x: 0,
            section_y: 0,
            section_z: 0,
            palette: &palette,
            packed_data: &packed,
            bits_per_block: 0,
            is_full_cube: &is_stone,
            get_block: &|_, _, _| None,
            texture_map: &texture_map,
            face_texture_map: &face_texture_map,
            atlas_width: 16,
            atlas_height: 16,
            tile_size: 16,
        };
        let mesh = build_section_mesh(&input);
        assert!(mesh.is_empty());
        assert_eq!(mesh.index_count, 0);
    }

    #[test]
    fn single_stone_block_has_faces() {
        // Секция 16x16x16, один камень в центре (8,8,8), остальное воздух
        let air_id = ResourceId::minecraft("air").unwrap();
        let stone_id = ResourceId::minecraft("stone").unwrap();
        let palette: Vec<ResourceId> = vec![air_id, stone_id];
        let mut packed: Vec<u64> = vec![0; 64]; // 4096 блоков * 1 бит / 64 = 64 слова
        // Установить бит для блока (8,8,8) = индекс палитры 1
        let pos = ((8 * 16 + 8) * 16 + 8) as usize;
        let bit_start = pos * 1;
        let word = bit_start / 64;
        let offset = bit_start % 64;
        packed[word] |= 1u64 << offset;

        let mut texture_map: TextureMap = HashMap::new();
        texture_map.insert(ResourceId::minecraft("stone").unwrap(), 0);
        texture_map.insert(ResourceId::minecraft("air").unwrap(), 0);
        let face_texture_map: Arc<FaceTextureMap> = Arc::new(HashMap::new());

        let input = SectionMeshInput {
            section_x: 0,
            section_y: 0,
            section_z: 0,
            palette: &palette,
            packed_data: &packed,
            bits_per_block: 1,
            is_full_cube: &is_stone,
            get_block: &move |wx, wy, wz| {
                // Только блок (8,8,8) — камень, остальное воздух
                if wx == 8 && wy == 8 && wz == 8 {
                    Some(ResourceId::minecraft("stone").unwrap())
                } else {
                    None
                }
            },
            texture_map: &texture_map,
            face_texture_map: &face_texture_map,
            atlas_width: 16,
            atlas_height: 16,
            tile_size: 16,
        };
        let mesh = build_section_mesh(&input);
        assert!(!mesh.is_empty());
        // Один изолированный куб = 6 граней = 12 треугольников = 36 индексов
        assert_eq!(mesh.index_count, 36);
        assert_eq!(mesh.vertices.len(), 24); // 6 граней * 4 вершины
    }

    #[test]
    fn adjacent_blocks_hide_shared_faces() {
        // Два соседних камня: (8,8,8) и (9,8,8)
        // Общая грань между ними скрыта: 2 куба × 5 граней × 6 индексов = 60
        let air_id = ResourceId::minecraft("air").unwrap();
        let stone_id = ResourceId::minecraft("stone").unwrap();
        let palette: Vec<ResourceId> = vec![air_id, stone_id];
        let mut packed: Vec<u64> = vec![0; 64];
        let set_bit = |packed: &mut Vec<u64>, x: i32, y: i32, z: i32| {
            let pos = ((y * 16 + z) * 16 + x) as usize;
            let bit_start = pos * 1;
            let word = bit_start / 64;
            let offset = bit_start % 64;
            packed[word] |= 1u64 << offset;
        };
        set_bit(&mut packed, 8, 8, 8);
        set_bit(&mut packed, 9, 8, 8);

        let mut texture_map: TextureMap = HashMap::new();
        texture_map.insert(ResourceId::minecraft("stone").unwrap(), 0);
        texture_map.insert(ResourceId::minecraft("air").unwrap(), 0);
        let face_texture_map: Arc<FaceTextureMap> = Arc::new(HashMap::new());

        let input = SectionMeshInput {
            section_x: 0,
            section_y: 0,
            section_z: 0,
            palette: &palette,
            packed_data: &packed,
            bits_per_block: 1,
            is_full_cube: &is_stone,
            get_block: &move |wx, wy, wz| {
                if (wx == 8 || wx == 9) && wy == 8 && wz == 8 {
                    Some(ResourceId::minecraft("stone").unwrap())
                } else {
                    None
                }
            },
            texture_map: &texture_map,
            face_texture_map: &face_texture_map,
            atlas_width: 16,
            atlas_height: 16,
            tile_size: 16,
        };
        let mesh = build_section_mesh(&input);
        assert!(!mesh.is_empty());
        // Каждый куб показывает 5 граней (общая скрыта) = 10 граней × 6 индексов = 60
        assert_eq!(mesh.index_count, 60);
    }

    #[test]
    fn face_normals_cover_all_six_directions() {
        let air_id = ResourceId::minecraft("air").unwrap();
        let stone_id = ResourceId::minecraft("stone").unwrap();
        let palette: Vec<ResourceId> = vec![air_id, stone_id];
        let mut packed: Vec<u64> = vec![0; 64];
        let pos = ((8 * 16 + 8) * 16 + 8) as usize;
        let bit_start = pos * 1;
        let word = bit_start / 64;
        let offset = bit_start % 64;
        packed[word] |= 1u64 << offset;

        let mut texture_map: TextureMap = HashMap::new();
        texture_map.insert(ResourceId::minecraft("stone").unwrap(), 0);
        texture_map.insert(ResourceId::minecraft("air").unwrap(), 0);
        let face_texture_map: Arc<FaceTextureMap> = Arc::new(HashMap::new());

        let input = SectionMeshInput {
            section_x: 0,
            section_y: 0,
            section_z: 0,
            palette: &palette,
            packed_data: &packed,
            bits_per_block: 1,
            is_full_cube: &is_stone,
            get_block: &|_, _, _| None,
            texture_map: &texture_map,
            face_texture_map: &face_texture_map,
            atlas_width: 16,
            atlas_height: 16,
            tile_size: 16,
        };
        let mesh = build_section_mesh(&input);

        // Ожидаемые нормали для 6 граней
        let expected: [[f32; 3]; 6] = [
            [0.0, -1.0, 0.0], // Down
            [0.0, 1.0, 0.0],  // Up
            [0.0, 0.0, -1.0], // North
            [0.0, 0.0, 1.0],  // South
            [-1.0, 0.0, 0.0], // West
            [1.0, 0.0, 0.0],  // East
        ];
        for &normal in &expected {
            assert!(
                mesh.vertices.iter().any(|v| v.normal == normal),
                "missing normal {:?}",
                normal
            );
        }
    }

    #[test]
    fn bits_per_block_zero_is_single_block() {
        // Вся секция один блок (индекс палитры 0 = камень)
        let stone_id = ResourceId::minecraft("stone").unwrap();
        let palette: Vec<ResourceId> = vec![stone_id];
        let packed: Vec<u64> = vec![0; 1]; // не используется при bits_per_block = 0

        let mut texture_map: TextureMap = HashMap::new();
        texture_map.insert(ResourceId::minecraft("stone").unwrap(), 0);

        let face_texture_map: Arc<FaceTextureMap> = Arc::new(HashMap::new());
        let input = SectionMeshInput {
            section_x: 5,
            section_y: 3,
            section_z: -2,
            palette: &palette,
            packed_data: &packed,
            bits_per_block: 0,
            is_full_cube: &is_stone,
            get_block: &|_, _, _| None,
            texture_map: &texture_map,
            face_texture_map: &face_texture_map,
            atlas_width: 16,
            atlas_height: 16,
            tile_size: 16,
        };
        let mesh = build_section_mesh(&input);
        assert!(!mesh.is_empty());
        assert_eq!(mesh.section_x, 5);
        assert_eq!(mesh.section_y, 3);
        assert_eq!(mesh.section_z, -2);
        // bits_per_block = 0: вся секция — один блок (камень).
        // 4096 блоков × 6 граней × 6 индексов = 147456 (все грани видны, так как get_block → None)
        assert_eq!(mesh.index_count, 147456);
    }

    // ==== Тесты для Phase 4 часть 2 (ревизии, очереди, инвалидация) ====

    #[test]
    fn mesh_revision_increment() {
        let r1 = MeshRevision(0);
        let r2 = r1.next();
        let r3 = r2.next();
        assert!(r1 < r2);
        assert!(r2 < r3);
        assert_eq!(r3.0, 2);
    }

    #[test]
    fn mesh_job_creation() {
        let palette: Vec<ResourceId> = vec![ResourceId::minecraft("stone").unwrap()];
        let packed: Vec<u64> = vec![0; 1];
        let mut texture_map: TextureMap = HashMap::new();
        texture_map.insert(ResourceId::minecraft("stone").unwrap(), 0);
        let input = SectionMeshInput {
            section_x: 1,
            section_y: 2,
            section_z: 3,
            palette: &palette,
            packed_data: &packed,
            bits_per_block: 0,
            is_full_cube: &|_| true,
            get_block: &|_, _, _| None,
            texture_map: &texture_map,
            face_texture_map: &face_texture_map,
            atlas_width: 16,
            atlas_height: 16,
            tile_size: 16,
        };
        let revision = MeshRevision(42);
        let job = make_mesh_job(&input, revision);

        assert_eq!(job.section_x, 1);
        assert_eq!(job.section_y, 2);
        assert_eq!(job.section_z, 3);
        assert_eq!(job.revision, MeshRevision(42));
        assert_eq!(job.palette.len(), 1);
        assert_eq!(job.bits_per_block, 0);
        assert_eq!(job.texture_map.get(&ResourceId::minecraft("stone").unwrap()), Some(&0));
    }

    #[test]
    fn mesh_result_staleness() {
        let mesh = ChunkMesh::empty(0, 0, 0);
        let result_new = MeshResult {
            mesh: mesh.clone(),
            revision: MeshRevision(5),
        };
        let result_old = MeshResult {
            mesh,
            revision: MeshRevision(3),
        };

        assert!(!is_mesh_stale(&result_new, MeshRevision(5)));
        assert!(is_mesh_stale(&result_old, MeshRevision(5)));
        assert!(!is_mesh_stale(&result_old, MeshRevision(3)));
    }

    #[test]
    fn neighbors_invalidation_center_block() {
        // Блок в центре секции (8,8,8) — соседи все внутри той же секции
        let sections = neighbors_for_invalidation(0, 0, 0, 8, 8, 8);
        // Только сама секция (0,0,0), так как все соседи внутри секции
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0], (0, 0, 0));
    }

    #[test]
    fn neighbors_invalidation_at_edge() {
        // Блок на границе секции (0,8,8) — сосед в секции (-1,0,0)
        let sections = neighbors_for_invalidation(0, 0, 0, 0, 8, 8);
        assert!(sections.contains(&(-1, 0, 0)));
        assert!(sections.contains(&(0, 0, 0)));
        assert!(!sections.contains(&(1, 0, 0)));
    }

    #[test]
    fn neighbors_invalidation_at_corner() {
        // Блок в углу (0,0,0) — соседи в трёх соседних секциях
        let sections = neighbors_for_invalidation(0, 0, 0, 0, 0, 0);
        assert!(sections.contains(&(-1, -1, -1)));
        assert!(sections.contains(&(0, 0, 0)));
        assert_eq!(sections.len(), 8); // 7 соседей + сама секция
    }
}