pub mod chunk;
pub mod config;
pub mod game_mode;
pub mod inventory;
pub mod player;
pub mod simulation;
pub mod types;
pub mod world;
pub mod ui;

use bevy::prelude::*;
use std::sync::Arc;
use game_mode::GameMode;
use world::{ActiveWorldGenerator, WorldPlugin};
use world::procedural::ProceduralGenerator;

/// Whether or not the game is paused.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Pause(pub bool);

/// A system set for systems that shouldn't run while the game is paused.
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PausableSystems;

/// High-level groupings of systems for the app in the `Update` schedule.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum AppSystems {
    /// Tick timers.
    TickTimers,
    /// Record player input.
    RecordInput,
    /// Do everything else.
    Update,
}

#[derive(Resource, Clone)]
pub struct Settings {
    pub master_volume: f32,
    pub mouse_sensitivity: f32,
    pub fov: f32,
    pub render_distance: u32,
    pub show_coordinates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            mouse_sensitivity: 1.0,
            fov: 90.0,
            render_distance: 8,
            show_coordinates: false,
        }
    }
}

/// Marker component for the UI camera used during non-gameplay screens.
#[derive(Component)]
pub struct UiCamera;

/// Spawn a 2D camera for UI rendering during splash/title screens.
pub fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((UiCamera, Camera2d));
}

/// Despawn the UI camera when entering gameplay (the 3D camera will take over).
pub fn despawn_ui_camera(mut commands: Commands, query: Query<Entity, With<UiCamera>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub struct VoxelGamePlugin;

impl Plugin for VoxelGamePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<inventory::Inventory>()
            .init_resource::<inventory::crafting::RecipeBook>()
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

        app.add_systems(
            Update,
            (
                inventory::ui::update_hotbar,
                inventory::ui::cycle_hotbar,
            )
                .in_set(PausableSystems),
        );
    }
}
