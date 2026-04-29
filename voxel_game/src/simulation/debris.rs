use bevy::prelude::*;
use crate::chunk::loading::ChunkedWorld;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{ChunkPos, LocalVoxelPos, VoxelId};

const GRAVITY: f32 = -9.8;
const SOLIDIFY_SECS: f32 = 5.0;

#[derive(Component)]
pub struct DebrisParticle {
    pub voxel_id: VoxelId,
    pub velocity: Vec3,
    pub rest_timer: f32,
}

impl DebrisParticle {
    pub fn new(voxel_id: VoxelId, velocity: Vec3) -> Self {
        Self { voxel_id, velocity, rest_timer: 0.0 }
    }
}

pub fn spawn_debris(
    commands: &mut Commands,
    voxel_id: VoxelId,
    world_pos: Vec3,
    velocity: Vec3,
) {
    commands.spawn((
        DebrisParticle::new(voxel_id, velocity),
        Transform::from_translation(world_pos),
        Visibility::Visible,
    ));
}

pub fn tick_debris(
    mut debris_query: Query<(Entity, &mut Transform, &mut DebrisParticle)>,
    world: Res<ChunkedWorld>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (_entity, mut transform, mut debris) in &mut debris_query {
        debris.velocity.y += GRAVITY * dt;

        let next_pos = transform.translation + debris.velocity * dt;

        let below = next_pos - Vec3::Y * VOXEL_SIZE;
        let floor_solid = is_world_pos_solid(&world, below);

        if floor_solid && debris.velocity.y < 0.0 {
            debris.velocity.y = 0.0;
            debris.velocity.x *= 0.8;
            debris.velocity.z *= 0.8;
        }

        let next_x = Vec3::new(next_pos.x, transform.translation.y, transform.translation.z);
        if is_world_pos_solid(&world, next_x) {
            debris.velocity.x = 0.0;
        }
        let next_z = Vec3::new(transform.translation.x, transform.translation.y, next_pos.z);
        if is_world_pos_solid(&world, next_z) {
            debris.velocity.z = 0.0;
        }

        let speed = debris.velocity.length();
        if speed < 0.05 {
            debris.velocity = Vec3::ZERO;
            debris.rest_timer += dt;
        } else {
            debris.rest_timer = 0.0;
        }

        transform.translation += debris.velocity * dt;
    }
}

pub fn solidify_resting_debris(
    mut commands: Commands,
    debris_query: Query<(Entity, &Transform, &DebrisParticle)>,
    mut world: ResMut<ChunkedWorld>,
) {
    let n = CHUNK_SIZE as i32;
    for (entity, transform, debris) in &debris_query {
        if debris.rest_timer < SOLIDIFY_SECS { continue; }

        let pos = transform.translation;
        let vx = (pos.x / VOXEL_SIZE).floor() as i32;
        let vy = (pos.y / VOXEL_SIZE).floor() as i32;
        let vz = (pos.z / VOXEL_SIZE).floor() as i32;

        let chunk_pos = ChunkPos(vx.div_euclid(n), vy.div_euclid(n), vz.div_euclid(n));
        let local = LocalVoxelPos::new(
            vx.rem_euclid(n) as u8,
            vy.rem_euclid(n) as u8,
            vz.rem_euclid(n) as u8,
        );

        if let Some(chunk) = world.get_mut(chunk_pos) {
            chunk.set(local, debris.voxel_id);
        }

        commands.entity(entity).despawn();
    }
}

fn is_world_pos_solid(world: &ChunkedWorld, pos: Vec3) -> bool {
    let n = CHUNK_SIZE as i32;
    let vx = (pos.x / VOXEL_SIZE).floor() as i32;
    let vy = (pos.y / VOXEL_SIZE).floor() as i32;
    let vz = (pos.z / VOXEL_SIZE).floor() as i32;
    let chunk_pos = ChunkPos(vx.div_euclid(n), vy.div_euclid(n), vz.div_euclid(n));
    let local = LocalVoxelPos::new(
        vx.rem_euclid(n) as u8,
        vy.rem_euclid(n) as u8,
        vz.rem_euclid(n) as u8,
    );
    world.get(chunk_pos).map_or(false, |c| c.is_solid(local))
}
