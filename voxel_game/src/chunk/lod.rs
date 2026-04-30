use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use bevy::tasks::Task;
use crate::chunk::meshing::MeshData;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{VoxelId, ChunkPos, AIR};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodLevel {
    Lod1,
    Lod2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct SuperChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub level: LodLevel,
}

impl SuperChunkPos {
    pub fn to_world_origin(self) -> Vec3 {
        let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let side = match self.level {
            LodLevel::Lod1 => 4.0 * lod0_side,
            LodLevel::Lod2 => 16.0 * lod0_side,
        };
        Vec3::new(self.x as f32 * side, self.y as f32 * side, self.z as f32 * side)
    }

    pub fn voxel_size(self) -> f32 {
        match self.level {
            LodLevel::Lod1 => VOXEL_SIZE * 4.0,
            LodLevel::Lod2 => VOXEL_SIZE * 16.0,
        }
    }

    pub fn from_world(pos: Vec3, level: LodLevel) -> Self {
        let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let side = match level {
            LodLevel::Lod1 => 4.0 * lod0_side,
            LodLevel::Lod2 => 16.0 * lod0_side,
        };
        SuperChunkPos {
            x: pos.x.div_euclid(side) as i32,
            y: pos.y.div_euclid(side) as i32,
            z: pos.z.div_euclid(side) as i32,
            level,
        }
    }

    /// The 64 LOD0 ChunkPos this LOD1 super-chunk covers, in cz/cy/cx order.
    pub fn lod0_chunks(self) -> Vec<ChunkPos> {
        debug_assert_eq!(self.level, LodLevel::Lod1, "lod0_chunks only valid for Lod1");
        let (bx, by, bz) = (self.x * 4, self.y * 4, self.z * 4);
        let mut out = Vec::with_capacity(64);
        for cz in 0..4i32 {
            for cy in 0..4i32 {
                for cx in 0..4i32 {
                    out.push(ChunkPos(bx + cx, by + cy, bz + cz));
                }
            }
        }
        out
    }

    /// The 64 LOD1 SuperChunkPos this LOD2 super-chunk covers, in cz/cy/cx order.
    pub fn lod1_super_chunks(self) -> Vec<SuperChunkPos> {
        debug_assert_eq!(self.level, LodLevel::Lod2, "lod1_super_chunks only valid for Lod2");
        let (bx, by, bz) = (self.x * 4, self.y * 4, self.z * 4);
        let mut out = Vec::with_capacity(64);
        for cz in 0..4i32 {
            for cy in 0..4i32 {
                for cx in 0..4i32 {
                    out.push(SuperChunkPos { x: bx + cx, y: by + cy, z: bz + cz, level: LodLevel::Lod1 });
                }
            }
        }
        out
    }
}

#[derive(Resource, Default)]
pub struct SuperChunkedWorld {
    pub chunks: HashMap<SuperChunkPos, crate::chunk::SuperChunk>,
}

#[derive(Resource, Default)]
pub struct PendingSuperChunks(pub VecDeque<SuperChunkPos>);

#[derive(Resource, Default)]
pub struct MeshingLodChunks(pub HashMap<SuperChunkPos, Task<(MeshData, Box<[VoxelId]>)>>);

#[derive(Resource, Default)]
pub struct SuperChunkEntities(pub HashMap<SuperChunkPos, Entity>);

/// Collapse 64 source voxel slices (a 4×4×4 grid of chunks, indexed cx + cy*4 + cz*16)
/// into a single CHUNK_SIZE³ voxel grid at `factor`× lower resolution.
/// Each output voxel is the most common non-air type in its factor³ source region,
/// or AIR if all source voxels are air.
pub fn downsample(sources: Vec<Box<[VoxelId]>>, factor: usize) -> Box<[VoxelId]> {
    debug_assert_eq!(sources.len(), factor * factor * factor,
        "sources must be factor³ chunks");
    let n = CHUNK_SIZE;
    let mut out = vec![AIR; n * n * n].into_boxed_slice();
    let mut counts: HashMap<VoxelId, u32> = HashMap::new();

    for oz in 0..n {
        for oy in 0..n {
            for ox in 0..n {
                counts.clear();

                for fz in 0..factor {
                    for fy in 0..factor {
                        for fx in 0..factor {
                            let sx = ox * factor + fx;
                            let sy = oy * factor + fy;
                            let sz = oz * factor + fz;
                            let cx = sx / n;
                            let cy = sy / n;
                            let cz = sz / n;
                            let ci = cx + cy * factor + cz * factor * factor;
                            let lx = sx % n;
                            let ly = sy % n;
                            let lz = sz % n;
                            let vi = lx + ly * n + lz * n * n;
                            let id = sources[ci][vi];
                            if id != AIR {
                                *counts.entry(id).or_insert(0) += 1;
                            }
                        }
                    }
                }

                out[ox + oy * n + oz * n * n] = counts
                    .iter()
                    .max_by_key(|(_, c)| *c)
                    .map(|(id, _)| *id)
                    .unwrap_or(AIR);
            }
        }
    }
    out
}

pub const LOD1_INNER: i32 = 10; // 10 * 12.8m = 128m
pub const LOD1_OUTER: i32 = 40; // 40 * 12.8m = 512m
pub const LOD1_Y_RADIUS: i32 = 5;

pub const LOD2_INNER: i32 = 10; // 10 * 51.2m = 512m
pub const LOD2_OUTER: i32 = 39; // 39 * 51.2m ≈ 2km
pub const LOD2_Y_RADIUS: i32 = 3;

pub fn lod_coordinator(
    player_query: Query<&Transform, With<crate::player::Player>>,
    mut super_world: ResMut<SuperChunkedWorld>,
    mut super_entities: ResMut<SuperChunkEntities>,
    mut pending: ResMut<PendingSuperChunks>,
    mut commands: Commands,
    mut last_lod1_pos: Local<Option<SuperChunkPos>>,
) {
    let Ok(player_tf) = player_query.single() else { return };
    let player_lod1 = SuperChunkPos::from_world(player_tf.translation, LodLevel::Lod1);
    let player_lod2 = SuperChunkPos::from_world(player_tf.translation, LodLevel::Lod2);

    if *last_lod1_pos == Some(player_lod1) {
        return;
    }
    *last_lod1_pos = Some(player_lod1);

    // Despawn entities and remove world data for out-of-range super-chunks
    let dead: Vec<SuperChunkPos> = super_entities
        .0
        .keys()
        .filter(|&&sp| {
            match sp.level {
                LodLevel::Lod1 => {
                    let xz = (sp.x - player_lod1.x).abs().max((sp.z - player_lod1.z).abs());
                    let dy = (sp.y - player_lod1.y).abs();
                    xz < LOD1_INNER || xz > LOD1_OUTER || dy > LOD1_Y_RADIUS
                }
                LodLevel::Lod2 => {
                    let xz = (sp.x - player_lod2.x).abs().max((sp.z - player_lod2.z).abs());
                    let dy = (sp.y - player_lod2.y).abs();
                    xz < LOD2_INNER || xz > LOD2_OUTER || dy > LOD2_Y_RADIUS
                }
            }
        })
        .copied()
        .collect();
    for sp in dead {
        if let Some(entity) = super_entities.0.remove(&sp) {
            commands.entity(entity).despawn();
        }
        super_world.chunks.remove(&sp);
    }

    // Rebuild pending queue
    pending.0.clear();

    // LOD1 ring
    for dx in -LOD1_OUTER..=LOD1_OUTER {
        for dy in -LOD1_Y_RADIUS..=LOD1_Y_RADIUS {
            for dz in -LOD1_OUTER..=LOD1_OUTER {
                let xz = dx.abs().max(dz.abs());
                if xz < LOD1_INNER || xz > LOD1_OUTER { continue; }
                let sp = SuperChunkPos {
                    x: player_lod1.x + dx,
                    y: player_lod1.y + dy,
                    z: player_lod1.z + dz,
                    level: LodLevel::Lod1,
                };
                if !super_world.chunks.contains_key(&sp) {
                    pending.0.push_back(sp);
                }
            }
        }
    }

    // LOD2 ring
    for dx in -LOD2_OUTER..=LOD2_OUTER {
        for dy in -LOD2_Y_RADIUS..=LOD2_Y_RADIUS {
            for dz in -LOD2_OUTER..=LOD2_OUTER {
                let xz = dx.abs().max(dz.abs());
                if xz < LOD2_INNER || xz > LOD2_OUTER { continue; }
                let sp = SuperChunkPos {
                    x: player_lod2.x + dx,
                    y: player_lod2.y + dy,
                    z: player_lod2.z + dz,
                    level: LodLevel::Lod2,
                };
                if !super_world.chunks.contains_key(&sp) {
                    pending.0.push_back(sp);
                }
            }
        }
    }

    // Sort: LOD1 before LOD2, surface-first within each level
    pending.0.make_contiguous().sort_unstable_by_key(|sp| {
        let level_cost: i32 = match sp.level { LodLevel::Lod1 => 0, LodLevel::Lod2 => 1_000_000 };
        let (dx, dy, dz) = match sp.level {
            LodLevel::Lod1 => (sp.x - player_lod1.x, sp.y - player_lod1.y, sp.z - player_lod1.z),
            LodLevel::Lod2 => (sp.x - player_lod2.x, sp.y - player_lod2.y, sp.z - player_lod2.z),
        };
        let xz = dx.abs() + dz.abs();
        let y_cost = if dy < 0 { (-dy) * 4 } else { dy };
        level_cost + xz + y_cost
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod1_super_chunk_world_origin() {
        let sp = SuperChunkPos { x: 1, y: 0, z: 0, level: LodLevel::Lod1 };
        let origin = sp.to_world_origin();
        // 1 LOD1 unit = 4 * 32 * 0.1 = 12.8m
        assert!((origin.x - 12.8).abs() < 1e-4, "x={}", origin.x);
        assert_eq!(origin.y, 0.0);
        assert_eq!(origin.z, 0.0);
    }

    #[test]
    fn lod2_super_chunk_world_origin() {
        let sp = SuperChunkPos { x: 1, y: 0, z: 0, level: LodLevel::Lod2 };
        let origin = sp.to_world_origin();
        // 1 LOD2 unit = 16 * 32 * 0.1 = 51.2m
        assert!((origin.x - 51.2).abs() < 1e-3, "x={}", origin.x);
    }

    #[test]
    fn lod1_voxel_size_is_4x() {
        let sp = SuperChunkPos { x: 0, y: 0, z: 0, level: LodLevel::Lod1 };
        assert!((sp.voxel_size() - 0.4).abs() < 1e-6);
    }

    #[test]
    fn lod2_voxel_size_is_16x() {
        let sp = SuperChunkPos { x: 0, y: 0, z: 0, level: LodLevel::Lod2 };
        assert!((sp.voxel_size() - 1.6).abs() < 1e-6);
    }

    #[test]
    fn lod0_chunks_returns_64() {
        let sp = SuperChunkPos { x: 0, y: 0, z: 0, level: LodLevel::Lod1 };
        let chunks = sp.lod0_chunks();
        assert_eq!(chunks.len(), 64);
    }

    #[test]
    fn lod0_chunks_correct_positions() {
        let sp = SuperChunkPos { x: 1, y: 0, z: 0, level: LodLevel::Lod1 };
        let chunks = sp.lod0_chunks();
        // base_x = 1 * 4 = 4; first chunk (cx=0,cy=0,cz=0) = (4, 0, 0)
        assert_eq!(chunks[0], crate::types::ChunkPos(4, 0, 0));
        // last chunk (cx=3,cy=3,cz=3): (4+3, 0+3, 0+3) = (7, 3, 3)
        assert_eq!(chunks[63], crate::types::ChunkPos(7, 3, 3));
        // These pin the documented cz/cy/cx iteration order
        assert_eq!(chunks[1],  crate::types::ChunkPos(5, 0, 0)); // cx=1,cy=0,cz=0
        assert_eq!(chunks[4],  crate::types::ChunkPos(4, 1, 0)); // cx=0,cy=1,cz=0
        assert_eq!(chunks[16], crate::types::ChunkPos(4, 0, 1)); // cx=0,cy=0,cz=1
    }

    #[test]
    fn lod1_super_chunks_returns_64() {
        let sp = SuperChunkPos { x: 0, y: 0, z: 0, level: LodLevel::Lod2 };
        let sub = sp.lod1_super_chunks();
        assert_eq!(sub.len(), 64);
        assert_eq!(sub[0].level, LodLevel::Lod1);
    }

    #[test]
    fn downsample_all_stone_stays_stone() {
        use crate::types::STONE;
        let n = CHUNK_SIZE;
        let solid: Box<[VoxelId]> = vec![STONE; n * n * n].into_boxed_slice();
        let sources: Vec<Box<[VoxelId]>> = (0..64).map(|_| solid.clone()).collect();
        let out = downsample(sources, 4);
        assert_eq!(out.len(), n * n * n);
        assert!(out.iter().all(|&v| v == STONE), "expected all stone");
    }

    #[test]
    fn downsample_all_air_stays_air() {
        let n = CHUNK_SIZE;
        let empty: Box<[VoxelId]> = vec![AIR; n * n * n].into_boxed_slice();
        let sources: Vec<Box<[VoxelId]>> = (0..64).map(|_| empty.clone()).collect();
        let out = downsample(sources, 4);
        assert!(out.iter().all(|&v| v == AIR), "expected all air");
    }

    #[test]
    fn downsample_majority_wins() {
        use crate::types::{STONE, DIRT};
        let n = CHUNK_SIZE;
        // Output voxel (0,0,0) samples source voxels (0..4, 0..4, 0..4).
        // That 4×4×4=64 cube lives entirely within chunk index 0 (cx=0,cy=0,cz=0).
        // Output voxel (8,0,0) samples source voxels (32..36, 0..4, 0..4).
        // sx=32 is in cx=1. So if sources[0] is all stone and the rest are dirt,
        // output voxel (0,0,0) = stone; output voxel (8,0,0) = dirt.
        let stone_chunk: Box<[VoxelId]> = vec![STONE; n * n * n].into_boxed_slice();
        let dirt_chunk: Box<[VoxelId]> = vec![DIRT; n * n * n].into_boxed_slice();
        let mut sources: Vec<Box<[VoxelId]>> = (0..64).map(|_| dirt_chunk.clone()).collect();
        sources[0] = stone_chunk;
        let out = downsample(sources, 4);
        assert_eq!(out[0], STONE, "voxel (0,0,0) should be stone");
        assert_eq!(out[8], DIRT,  "voxel (8,0,0) should be dirt");
    }

    #[test]
    fn downsample_cross_chunk_boundary() {
        use crate::types::{STONE, DIRT};
        let n = CHUNK_SIZE;
        let stone_chunk: Box<[VoxelId]> = vec![STONE; n * n * n].into_boxed_slice();
        let dirt_chunk: Box<[VoxelId]> = vec![DIRT; n * n * n].into_boxed_slice();
        // chunks[cx=0,...] = stone, chunks[cx=1,2,3,...] = dirt
        // chunk index: ci = cx + cy*4 + cz*16
        let sources: Vec<Box<[VoxelId]>> = (0..64).map(|i| {
            let cx = i % 4;
            if cx == 0 { stone_chunk.clone() } else { dirt_chunk.clone() }
        }).collect();
        let out = downsample(sources, 4);
        // ox=0..7: source sx=0..31 → all cx=0 → stone
        // ox=8..15: source sx=32..63 → all cx=1 → dirt
        assert_eq!(out[0], STONE, "ox=0 should be stone");
        assert_eq!(out[7], STONE, "ox=7 should be stone (last voxel in cx=0)");
        assert_eq!(out[8], DIRT,  "ox=8 should be dirt (first voxel in cx=1)");
    }

    #[test]
    fn lod1_ring_inner_outer_match_meters() {
        let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let lod1_side = 4.0 * lod0_side;
        assert!((LOD1_INNER as f32 * lod1_side - 128.0).abs() < 0.1);
        assert!((LOD1_OUTER as f32 * lod1_side - 512.0).abs() < 0.1);
    }

    #[test]
    fn lod2_ring_inner_outer_match_meters() {
        let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let lod2_side = 16.0 * lod0_side;
        assert!((LOD2_INNER as f32 * lod2_side - 512.0).abs() < 0.1);
        assert!(LOD2_OUTER as f32 * lod2_side >= 1800.0);
    }
}
