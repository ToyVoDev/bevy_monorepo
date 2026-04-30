use bevy::prelude::*;
use avian3d::prelude::*;
use super::Player;
use super::camera::PlayerCamera;
use crate::chunk::loading::ChunkedWorld;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{ChunkPos, LocalVoxelPos};
use crate::Settings;
use crate::ui::screens::Screen;

/// Scans the Y column at spawn XZ (chunk 0,_,0 local voxel 5,_,5) top-down
/// and returns 1m above the first solid voxel, or 5.0 if none is found.
pub fn find_spawn_y(world: &ChunkedWorld, spawn_radius: u32) -> f32 {
    let r = spawn_radius as i32;
    for chunk_y in (-r..=r).rev() {
        let chunk_pos = ChunkPos(0, chunk_y, 0);
        if let Some(chunk) = world.get(chunk_pos) {
            for local_y in (0..CHUNK_SIZE).rev() {
                let vp = LocalVoxelPos::new(5, local_y as u8, 5);
                if chunk.is_solid(vp) {
                    let world_y = (chunk_y as f32 * CHUNK_SIZE as f32
                        + local_y as f32
                        + 1.0)
                        * VOXEL_SIZE
                        + 1.0;
                    return world_y;
                }
            }
        }
    }
    5.0
}

pub fn spawn_player(
    mut commands: Commands,
    world: Res<ChunkedWorld>,
    settings: Res<Settings>,
) {
    let y = find_spawn_y(&world, settings.spawn_radius);
    commands.spawn((
        Player,
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.9),
        LockedAxes::ROTATION_LOCKED,
        LinearDamping(0.0),
        Friction::ZERO,
        Restitution::ZERO,
        GravityScale(1.0),
        LinearVelocity::default(),
        Transform::from_xyz(0.5, y, 0.5),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::loading::ChunkedWorld;
    use crate::chunk::Chunk;
    use crate::types::{ChunkPos, LocalVoxelPos, STONE};

    #[test]
    fn find_spawn_y_finds_surface() {
        let mut world = ChunkedWorld::default();
        let mut chunk = Chunk::new();
        chunk.set(LocalVoxelPos::new(5, 10, 5), STONE);
        world.chunks.insert(ChunkPos(0, 0, 0), chunk);
        let y = find_spawn_y(&world, 3);
        // World Y of top face = (0*32 + 10 + 1)*0.1 + 1.0 = 1.1 + 1.0 = 2.1
        assert!((y - 2.1).abs() < 1e-4, "expected 2.1, got {y}");
    }

    #[test]
    fn find_spawn_y_fallback_when_no_solid() {
        let world = ChunkedWorld::default();
        assert_eq!(find_spawn_y(&world, 3), 5.0);
    }

    #[test]
    fn find_spawn_y_prefers_higher_chunk() {
        let mut world = ChunkedWorld::default();
        let mut upper = Chunk::new();
        upper.set(LocalVoxelPos::new(5, 0, 5), STONE);
        world.chunks.insert(ChunkPos(0, 1, 0), upper);
        let mut lower = Chunk::new();
        lower.set(LocalVoxelPos::new(5, 31, 5), STONE);
        world.chunks.insert(ChunkPos(0, 0, 0), lower);
        let y = find_spawn_y(&world, 3);
        // chunk (0,1,0), local_y=0: (1*32 + 0 + 1)*0.1 + 1.0 = 3.3 + 1.0 = 4.3
        assert!((y - 4.3).abs() < 1e-4, "expected 4.3, got {y}");
    }
}
