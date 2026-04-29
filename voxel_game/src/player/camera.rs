use bevy::prelude::*;
use super::Player;

#[derive(Component)]
pub struct PlayerCamera;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.8, 0.0),
    ));
}

pub fn sync_camera(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut yaw: Local<f32>,
    mut pitch: Local<f32>,
) {
    let Ok(player_transform) = player_query.single() else { return };
    let Ok(mut cam_transform) = camera_query.single_mut() else { return };

    let sensitivity = 0.002_f32;
    for event in mouse_motion.read() {
        *yaw -= event.delta.x * sensitivity;
        *pitch -= event.delta.y * sensitivity;
        *pitch = pitch.clamp(-1.5, 1.5);
    }

    cam_transform.translation = player_transform.translation + Vec3::Y * 0.8;
    cam_transform.rotation = Quat::from_euler(EulerRot::YXZ, *yaw, *pitch, 0.0);
}
