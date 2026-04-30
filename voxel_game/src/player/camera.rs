use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use super::Player;
use crate::Settings;
use crate::ui::screens::Screen;

#[derive(Component)]
pub struct PlayerCamera;

pub fn spawn_camera(
    mut commands: Commands,
    settings: Res<Settings>,
) {
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: settings.fov.to_radians(),
            ..default()
        }),
        Transform::from_xyz(0.0, 0.8, 0.0),
        DespawnOnExit(Screen::Gameplay),
    ));
}

pub fn sync_camera(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    settings: Res<Settings>,
    mut yaw: Local<f32>,
    mut pitch: Local<f32>,
) {
    let Ok(player_transform) = player_query.single() else { return };
    let Ok(mut cam_transform) = camera_query.single_mut() else { return };

    let sensitivity = 0.002_f32 * settings.mouse_sensitivity;
    if mouse_motion.delta != Vec2::ZERO {
        *yaw -= mouse_motion.delta.x * sensitivity;
        *pitch -= mouse_motion.delta.y * sensitivity;
        *pitch = pitch.clamp(-1.5, 1.5);
    }

    cam_transform.translation = player_transform.translation + Vec3::Y * 0.8;
    cam_transform.rotation = Quat::from_euler(EulerRot::YXZ, *yaw, *pitch, 0.0);
}
