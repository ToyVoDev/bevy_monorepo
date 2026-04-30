use bevy::prelude::*;
use avian3d::prelude::*;
use super::Player;
use super::camera::PlayerCamera;
use crate::ui::screens::Screen;

pub fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.9), // radius=0.4, length=0.9
        LockedAxes::ROTATION_LOCKED,
        LinearDamping(0.0),
        Friction::ZERO,
        Restitution::ZERO,
        GravityScale(1.0),
        LinearVelocity::default(),
        Transform::from_xyz(0.5, 5.0, 0.5),
        Visibility::Hidden,
        DespawnOnExit(Screen::Gameplay),
    ));
}

pub fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, With<PlayerCamera>>,
    mut player_query: Query<(&Transform, &mut LinearVelocity), With<Player>>,
) {
    let Ok(cam_transform) = camera_query.single() else { return };
    let Ok((_, mut velocity)) = player_query.single_mut() else { return };

    let speed = 5.0_f32;
    let jump_impulse = 7.0_f32;

    let forward = Vec3::new(cam_transform.forward().x, 0.0, cam_transform.forward().z).normalize_or_zero();
    let right = Vec3::new(cam_transform.right().x, 0.0, cam_transform.right().z).normalize_or_zero();

    let mut wish_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { wish_dir += forward; }
    if keys.pressed(KeyCode::KeyS) { wish_dir -= forward; }
    if keys.pressed(KeyCode::KeyA) { wish_dir -= right; }
    if keys.pressed(KeyCode::KeyD) { wish_dir += right; }

    let wish_dir = wish_dir.normalize_or_zero();
    velocity.x = wish_dir.x * speed;
    velocity.z = wish_dir.z * speed;

    if keys.just_pressed(KeyCode::Space) && velocity.y.abs() < 0.1 {
        velocity.y = jump_impulse;
    }
}
