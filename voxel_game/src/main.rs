use bevy::{asset::AssetMetaCheck, prelude::*};
use voxel_game::VoxelGamePlugin;
use voxel_game::ui;
use voxel_game::ui::screens::Screen;
use voxel_game::{AppSystems, Pause, PausableSystems};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Voxel Game".into(),
                        ..default()
                    }),
                    ..default()
                }),
            AppPlugin,
        ))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 500.0,
            affects_lightmapped_meshes: true,
        })
        .run();
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ui::plugin,
            VoxelGamePlugin,
        ));

        // Spawn UI camera for splash/title screens.
        app.add_systems(Startup, voxel_game::spawn_ui_camera);

        // Set up the `Pause` state.
        app.init_state::<Pause>();
        app.configure_sets(Update, PausableSystems.run_if(in_state(Pause(false)).and(in_state(Screen::Gameplay))));

        // Order new `AppSystems` variants by adding them here:
        app.configure_sets(
            Update,
            (
                AppSystems::TickTimers,
                AppSystems::RecordInput,
                AppSystems::Update,
            )
                .chain(),
        );

    }
}



