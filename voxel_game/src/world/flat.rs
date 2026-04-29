use crate::chunk::Chunk;
use crate::config::CHUNK_SIZE;
use crate::types::{ChunkPos, LocalVoxelPos, VoxelId, AIR};
use super::WorldGenerator;

/// Flat-world generator: all voxels with `world_y < surface_y_voxels` are filled,
/// all voxels at or above are air.
#[derive(Debug, Clone)]
pub struct FlatGenerator {
    /// World-voxel Y of the surface (exclusive upper bound of solid fill).
    /// `surface_y_voxels = 0` means the first solid layer is at world_y = -1.
    pub surface_y_voxels: i32,
    pub fill_material: VoxelId,
}

impl FlatGenerator {
    pub fn new(surface_y_voxels: i32, fill_material: VoxelId) -> Self {
        Self { surface_y_voxels, fill_material }
    }
}

impl WorldGenerator for FlatGenerator {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let n = CHUNK_SIZE as i32;
        let mut chunk = Chunk::new();
        for z in 0..n as u8 {
            for x in 0..n as u8 {
                for y in 0..n as u8 {
                    let world_y = pos.1 * n + y as i32;
                    let voxel = if world_y < self.surface_y_voxels {
                        self.fill_material
                    } else {
                        AIR
                    };
                    chunk.set(LocalVoxelPos::new(x, y, z), voxel);
                }
            }
        }
        chunk
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::STONE;

    fn make_gen() -> FlatGenerator {
        FlatGenerator::new(0, STONE)
    }

    #[test]
    fn chunk_below_surface_is_solid() {
        let g = make_gen();
        let chunk = g.generate_chunk(ChunkPos(0, -1, 0));
        let pos = LocalVoxelPos::new(0, 0, 0);
        assert_eq!(chunk.get(pos), STONE, "chunk below surface should be solid");
    }

    #[test]
    fn chunk_above_surface_is_air() {
        let g = make_gen();
        let chunk = g.generate_chunk(ChunkPos(0, 1, 0));
        let pos = LocalVoxelPos::new(0, 0, 0);
        assert_eq!(chunk.get(pos), AIR, "chunk above surface should be air");
    }

    #[test]
    fn surface_chunk_has_mixed_content() {
        // surface_y_voxels=16: local y < 16 solid, y >= 16 air, chunk_y=0.
        let g = FlatGenerator::new(16, STONE);
        let chunk = g.generate_chunk(ChunkPos(0, 0, 0));
        assert_eq!(chunk.get(LocalVoxelPos::new(0, 15, 0)), STONE, "y=15 should be solid");
        assert_eq!(chunk.get(LocalVoxelPos::new(0, 16, 0)), AIR,   "y=16 should be air");
    }
}
