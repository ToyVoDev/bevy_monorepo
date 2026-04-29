pub mod chunk;
pub mod config;
pub mod game_mode;
pub mod player;
pub mod types;
pub mod world;

use bevy::prelude::*;
use game_mode::GameMode;
use world::{ActiveWorldGenerator, WorldPlugin};
use world::flat::FlatGenerator;
use types::STONE;

pub struct VoxelGamePlugin;

impl Plugin for VoxelGamePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(GameMode::Creative)
            .insert_resource(ActiveWorldGenerator(Box::new(
                FlatGenerator::new(0, STONE),
            )))
            .add_plugins((
                chunk::ChunkPlugin,
                WorldPlugin,
                player::PlayerPlugin,
            ));
    }
}
