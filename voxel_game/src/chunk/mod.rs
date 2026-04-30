pub mod meshing;
pub mod loading;
pub mod rendering;
pub mod lod;

use bevy::prelude::*;
use crate::config::CHUNK_SIZE;
use crate::PausableSystems;
use crate::types::{VoxelId, LocalVoxelPos, AIR};
use loading::{ChunkedWorld, PendingGeneration, GeneratingChunks,
              load_unload_chunks, spawn_generation_tasks, collect_generated_chunks};
use rendering::{ChunkEntities, MeshingChunks, spawn_meshing_tasks, collect_meshed_chunks};

pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ChunkedWorld>()
            .init_resource::<ChunkEntities>()
            .init_resource::<PendingGeneration>()
            .init_resource::<GeneratingChunks>()
            .init_resource::<MeshingChunks>()
            .add_systems(Update, (
                load_unload_chunks,
                spawn_generation_tasks.after(load_unload_chunks),
                collect_generated_chunks.after(spawn_generation_tasks),
                spawn_meshing_tasks.after(collect_generated_chunks),
                collect_meshed_chunks.after(spawn_meshing_tasks),
            ).in_set(PausableSystems));
    }
}

#[derive(Debug)]
pub struct Chunk {
    pub voxels: Box<[VoxelId]>,
    pub dirty: bool,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            voxels: vec![AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE].into_boxed_slice(),
            dirty: true,
        }
    }

    pub fn get(&self, pos: LocalVoxelPos) -> VoxelId {
        self.voxels[pos.to_index()]
    }

    pub fn set(&mut self, pos: LocalVoxelPos, id: VoxelId) {
        self.voxels[pos.to_index()] = id;
        self.dirty = true;
    }

    pub fn is_solid(&self, pos: LocalVoxelPos) -> bool {
        self.get(pos) != AIR
    }
}

pub struct SuperChunk {
    pub voxels: Box<[VoxelId]>,
}

impl SuperChunk {
    pub fn new() -> Self {
        let n = CHUNK_SIZE;
        Self { voxels: vec![AIR; n * n * n].into_boxed_slice() }
    }
}

impl Default for SuperChunk {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chunk_is_all_air() {
        let c = Chunk::new();
        assert_eq!(c.get(LocalVoxelPos::new(0, 0, 0)), AIR);
        assert_eq!(c.get(LocalVoxelPos::new(31, 31, 31)), AIR);
    }

    #[test]
    fn set_get_voxel() {
        let mut c = Chunk::new();
        let pos = LocalVoxelPos::new(1, 2, 3);
        c.set(pos, crate::types::STONE);
        assert_eq!(c.get(pos), crate::types::STONE);
        assert_eq!(c.get(LocalVoxelPos::new(0, 0, 0)), AIR);
    }

    #[test]
    fn set_marks_dirty() {
        let mut c = Chunk::new();
        assert!(c.dirty);
        c.dirty = false;
        c.set(LocalVoxelPos::new(0, 0, 0), crate::types::STONE);
        assert!(c.dirty);
    }

    #[test]
    fn is_solid_reflects_voxel_content() {
        let mut c = Chunk::new();
        let pos = LocalVoxelPos::new(4, 4, 4);
        assert!(!c.is_solid(pos));
        c.set(pos, crate::types::STONE);
        assert!(c.is_solid(pos));
    }
}
