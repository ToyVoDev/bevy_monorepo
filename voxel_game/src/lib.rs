pub mod chunk;
pub mod config;
pub mod game_mode;
pub mod inventory;
pub mod player;
pub mod simulation;
pub mod types;
pub mod world;

use bevy::prelude::*;
use std::sync::Arc;
use game_mode::GameMode;
use world::{ActiveWorldGenerator, WorldPlugin};
use world::procedural::ProceduralGenerator;

pub struct VoxelGamePlugin;

impl Plugin for VoxelGamePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<inventory::Inventory>()
            .init_resource::<inventory::crafting::RecipeBook>()
            .add_systems(Startup, inventory::crafting::load_recipes)
            .add_systems(Startup, inventory::ui::spawn_hotbar)
            .add_systems(Update, (
                inventory::ui::update_hotbar,
                inventory::ui::cycle_hotbar,
            ))
            .insert_resource(GameMode::Creative)
            .insert_resource(ActiveWorldGenerator(Arc::new(
                ProceduralGenerator::new(12345),
            )))
            .add_plugins((
                chunk::ChunkPlugin,
                WorldPlugin,
                player::PlayerPlugin,
                simulation::SimulationPlugin,
            ));
    }
}
