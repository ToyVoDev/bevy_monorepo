pub mod flat;

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
    fn build(&self, _app: &mut App) {}
}
