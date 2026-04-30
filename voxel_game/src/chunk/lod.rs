use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use bevy::tasks::Task;
use crate::chunk::meshing::MeshData;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{VoxelId, ChunkPos};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodLevel {
    Lod1,
    Lod2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct SuperChunkPos(pub i32, pub i32, pub i32, pub LodLevel);

impl SuperChunkPos {
    pub fn to_world_origin(self) -> Vec3 {
        let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let side = match self.3 {
            LodLevel::Lod1 => 4.0 * lod0_side,
            LodLevel::Lod2 => 16.0 * lod0_side,
        };
        Vec3::new(self.0 as f32 * side, self.1 as f32 * side, self.2 as f32 * side)
    }

    pub fn voxel_size(self) -> f32 {
        match self.3 {
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
        SuperChunkPos(
            pos.x.div_euclid(side) as i32,
            pos.y.div_euclid(side) as i32,
            pos.z.div_euclid(side) as i32,
            level,
        )
    }

    /// The 64 LOD0 ChunkPos this LOD1 super-chunk covers, in cz/cy/cx order.
    pub fn lod0_chunks(self) -> Vec<ChunkPos> {
        assert_eq!(self.3, LodLevel::Lod1);
        let (bx, by, bz) = (self.0 * 4, self.1 * 4, self.2 * 4);
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
        assert_eq!(self.3, LodLevel::Lod2);
        let (bx, by, bz) = (self.0 * 4, self.1 * 4, self.2 * 4);
        let mut out = Vec::with_capacity(64);
        for cz in 0..4i32 {
            for cy in 0..4i32 {
                for cx in 0..4i32 {
                    out.push(SuperChunkPos(bx + cx, by + cy, bz + cz, LodLevel::Lod1));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod1_super_chunk_world_origin() {
        let sp = SuperChunkPos(1, 0, 0, LodLevel::Lod1);
        let origin = sp.to_world_origin();
        // 1 LOD1 unit = 4 * 32 * 0.1 = 12.8m
        assert!((origin.x - 12.8).abs() < 1e-4, "x={}", origin.x);
        assert_eq!(origin.y, 0.0);
        assert_eq!(origin.z, 0.0);
    }

    #[test]
    fn lod2_super_chunk_world_origin() {
        let sp = SuperChunkPos(1, 0, 0, LodLevel::Lod2);
        let origin = sp.to_world_origin();
        // 1 LOD2 unit = 16 * 32 * 0.1 = 51.2m
        assert!((origin.x - 51.2).abs() < 1e-3, "x={}", origin.x);
    }

    #[test]
    fn lod1_voxel_size_is_4x() {
        let sp = SuperChunkPos(0, 0, 0, LodLevel::Lod1);
        assert!((sp.voxel_size() - 0.4).abs() < 1e-6);
    }

    #[test]
    fn lod2_voxel_size_is_16x() {
        let sp = SuperChunkPos(0, 0, 0, LodLevel::Lod2);
        assert!((sp.voxel_size() - 1.6).abs() < 1e-6);
    }

    #[test]
    fn lod0_chunks_returns_64() {
        let sp = SuperChunkPos(0, 0, 0, LodLevel::Lod1);
        let chunks = sp.lod0_chunks();
        assert_eq!(chunks.len(), 64);
    }

    #[test]
    fn lod0_chunks_correct_positions() {
        let sp = SuperChunkPos(1, 0, 0, LodLevel::Lod1);
        let chunks = sp.lod0_chunks();
        // base_x = 1 * 4 = 4; first chunk (cx=0,cy=0,cz=0) = (4, 0, 0)
        assert_eq!(chunks[0], crate::types::ChunkPos(4, 0, 0));
        // last chunk (cx=3,cy=3,cz=3): (4+3, 0+3, 0+3) = (7, 3, 3)
        assert_eq!(chunks[63], crate::types::ChunkPos(7, 3, 3));
    }

    #[test]
    fn lod1_super_chunks_returns_64() {
        let sp = SuperChunkPos(0, 0, 0, LodLevel::Lod2);
        let sub = sp.lod1_super_chunks();
        assert_eq!(sub.len(), 64);
        assert_eq!(sub[0].3, LodLevel::Lod1);
    }
}
