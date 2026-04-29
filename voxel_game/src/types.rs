use bevy::prelude::*;

pub type VoxelId = u16;
pub const AIR: VoxelId = 0;
pub const STONE: VoxelId = 1;
pub const DIRT: VoxelId = 2;
pub const TOPSOIL: VoxelId = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct ChunkPos(pub i32, pub i32, pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVoxelPos {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl ChunkPos {
    pub fn to_world_origin(self) -> Vec3 {
        let s = crate::config::CHUNK_SIZE as f32 * crate::config::VOXEL_SIZE;
        Vec3::new(self.0 as f32 * s, self.1 as f32 * s, self.2 as f32 * s)
    }

    pub fn from_world(pos: Vec3) -> Self {
        // s = CHUNK_SIZE * VOXEL_SIZE is not exactly representable in f32 when
        // VOXEL_SIZE is 0.1. Positions accumulated via repeated addition can drift
        // by ~1e-6 at chunk boundaries. Track this if seam artifacts appear in
        // chunk loading (Task 6+).
        let s = crate::config::CHUNK_SIZE as f32 * crate::config::VOXEL_SIZE;
        ChunkPos(
            pos.x.div_euclid(s) as i32,
            pos.y.div_euclid(s) as i32,
            pos.z.div_euclid(s) as i32,
        )
    }
}

impl LocalVoxelPos {
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        assert!((x as usize) < crate::config::CHUNK_SIZE, "x={x} out of bounds for CHUNK_SIZE={}", crate::config::CHUNK_SIZE);
        assert!((y as usize) < crate::config::CHUNK_SIZE, "y={y} out of bounds for CHUNK_SIZE={}", crate::config::CHUNK_SIZE);
        assert!((z as usize) < crate::config::CHUNK_SIZE, "z={z} out of bounds for CHUNK_SIZE={}", crate::config::CHUNK_SIZE);
        Self { x, y, z }
    }

    pub fn to_index(self) -> usize {
        let n = crate::config::CHUNK_SIZE;
        self.x as usize + self.y as usize * n + self.z as usize * n * n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_voxel_pos_index_roundtrip() {
        let pos = LocalVoxelPos { x: 3, y: 5, z: 7 };
        let idx = pos.to_index();
        use crate::config::CHUNK_SIZE;
        let x = (idx % CHUNK_SIZE) as u8;
        let y = ((idx / CHUNK_SIZE) % CHUNK_SIZE) as u8;
        let z = (idx / (CHUNK_SIZE * CHUNK_SIZE)) as u8;
        assert_eq!((x, y, z), (pos.x, pos.y, pos.z));
    }

    #[test]
    fn chunk_pos_from_world_origin() {
        use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
        let chunk_world = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let pos = Vec3::new(chunk_world, 0.0, 0.0);
        assert_eq!(ChunkPos::from_world(pos), ChunkPos(1, 0, 0));
    }

    #[test]
    fn chunk_pos_to_world_origin_roundtrip() {
        let cp = ChunkPos(2, -1, 3);
        let world = cp.to_world_origin();
        assert_eq!(ChunkPos::from_world(world), cp);
    }
}
