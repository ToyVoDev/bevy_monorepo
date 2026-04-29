use bevy::prelude::*;
use voxel_game::VoxelGamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxel Game".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VoxelGamePlugin)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 500.0,
            ..default()
        })
        .add_systems(Startup, setup_light)
        .run();
}

fn setup_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ, -0.5, 0.3, 0.0,
        )),
    ));
}
