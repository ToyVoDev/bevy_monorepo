pub mod flat;
pub mod procedural;

use bevy::prelude::*;
use crate::chunk::Chunk;
use crate::types::ChunkPos;

pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk;
}

#[derive(Resource)]
pub struct ActiveWorldGenerator(pub Box<dyn WorldGenerator>);

pub struct WorldPlugin;
impl Plugin for WorldPlugin {
    fn build(&self, _app: &mut App) {
        // ActiveWorldGenerator is inserted by the host plugin (e.g. VoxelGamePlugin)
        // to allow callers to configure the generator before adding this plugin.
    }
}
