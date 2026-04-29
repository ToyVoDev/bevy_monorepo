pub mod flat;
pub mod procedural;

use bevy::prelude::*;
use std::sync::Arc;
use crate::chunk::Chunk;
use crate::types::ChunkPos;

pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk;
}

#[derive(Resource, Clone)]
pub struct ActiveWorldGenerator(pub Arc<dyn WorldGenerator>);

pub struct WorldPlugin;
impl Plugin for WorldPlugin {
    fn build(&self, _app: &mut App) {}
}
