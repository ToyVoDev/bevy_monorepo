use noise::{NoiseFn, Fbm, Perlin, MultiFractal};
use crate::chunk::Chunk;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{ChunkPos, LocalVoxelPos, AIR, STONE, DIRT, TOPSOIL};
use super::WorldGenerator;

pub struct ProceduralGenerator {
    pub seed: u32,
    pub surface_amplitude: f64,
    pub surface_base_y: f64,
    pub cave_threshold: f64,
    surface_noise: Fbm<Perlin>,
    cave_noise: Perlin,
}

impl ProceduralGenerator {
    pub fn new(seed: u32) -> Self {
        let surface_noise: Fbm<Perlin> = Fbm::new(seed)
            .set_octaves(4)
            .set_frequency(0.005);
        let cave_noise = Perlin::new(seed.wrapping_add(1));
        Self {
            seed,
            surface_amplitude: 40.0,
            surface_base_y: 0.0,
            cave_threshold: 0.65,
            surface_noise,
            cave_noise,
        }
    }
}

impl WorldGenerator for ProceduralGenerator {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let n = CHUNK_SIZE as i32;
        let mut chunk = Chunk::new();

        for lz in 0..n as u8 {
            for lx in 0..n as u8 {
                let wx = pos.0 * n + lx as i32;
                let wz = pos.2 * n + lz as i32;

                let nx = wx as f64 * VOXEL_SIZE as f64;
                let nz = wz as f64 * VOXEL_SIZE as f64;
                let height_offset = self.surface_noise.get([nx, nz]) * self.surface_amplitude;
                let surface_voxel_y = (self.surface_base_y + height_offset) as i32;

                for ly in 0..n as u8 {
                    let wy = pos.1 * n + ly as i32;

                    let voxel = if wy > surface_voxel_y {
                        AIR
                    } else if wy == surface_voxel_y {
                        TOPSOIL
                    } else if wy > surface_voxel_y - 3 {
                        DIRT
                    } else {
                        let cv = self.cave_noise.get([
                            wx as f64 * VOXEL_SIZE as f64 * 0.5,
                            wy as f64 * VOXEL_SIZE as f64 * 0.5,
                            wz as f64 * VOXEL_SIZE as f64 * 0.5,
                        ]);
                        if cv.abs() > self.cave_threshold { AIR } else { STONE }
                    };

                    chunk.set(LocalVoxelPos::new(lx, ly, lz), voxel);
                }
            }
        }

        chunk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gen() -> ProceduralGenerator { ProceduralGenerator::new(42) }

    #[test]
    fn high_chunk_is_mostly_air() {
        let g = make_gen();
        let chunk = g.generate_chunk(ChunkPos(0, 20, 0));
        let air_count = chunk.voxels.iter().filter(|&&v| v == AIR).count();
        let total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
        assert!(air_count > total * 9 / 10, "high chunk should be >90% air, got {air_count}/{total}");
    }

    #[test]
    fn deep_chunk_is_mostly_solid() {
        let g = make_gen();
        let chunk = g.generate_chunk(ChunkPos(0, -10, 0));
        let solid_count = chunk.voxels.iter().filter(|&&v| v != AIR).count();
        let total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
        assert!(solid_count > total / 2, "deep chunk should be >50% solid, got {solid_count}/{total}");
    }

    #[test]
    fn surface_chunk_has_mixed_content() {
        let g = make_gen();
        let chunk = g.generate_chunk(ChunkPos(0, 0, 0));
        let has_air = chunk.voxels.iter().any(|&v| v == AIR);
        let has_solid = chunk.voxels.iter().any(|&v| v != AIR);
        assert!(has_air && has_solid, "surface chunk should have mixed air and solid");
    }
}
