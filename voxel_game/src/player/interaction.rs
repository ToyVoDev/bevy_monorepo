use bevy::prelude::*;
use crate::chunk::loading::ChunkedWorld;
use crate::config::VOXEL_SIZE;
use crate::types::{ChunkPos, LocalVoxelPos, VoxelId, AIR};

pub const REACH: f32 = 5.0;

#[derive(Debug, PartialEq)]
pub struct VoxelHit {
    pub chunk: ChunkPos,
    pub local: LocalVoxelPos,
    pub voxel_id: VoxelId,
    pub normal: IVec3,
}

pub fn raycast_voxels(
    world: &ChunkedWorld,
    origin: Vec3,
    direction: Vec3,
    max_dist: f32,
) -> Option<VoxelHit> {
    let dir = direction.normalize();
    let mut pos = (origin / VOXEL_SIZE).floor().as_ivec3();
    let step = IVec3::new(
        if dir.x >= 0.0 { 1 } else { -1 },
        if dir.y >= 0.0 { 1 } else { -1 },
        if dir.z >= 0.0 { 1 } else { -1 },
    );
    let delta = Vec3::new(
        if dir.x.abs() < f32::EPSILON { f32::INFINITY } else { VOXEL_SIZE / dir.x.abs() },
        if dir.y.abs() < f32::EPSILON { f32::INFINITY } else { VOXEL_SIZE / dir.y.abs() },
        if dir.z.abs() < f32::EPSILON { f32::INFINITY } else { VOXEL_SIZE / dir.z.abs() },
    );
    let origin_voxel = (origin / VOXEL_SIZE).floor();
    let mut t_max = Vec3::new(
        if dir.x.abs() < f32::EPSILON { f32::MAX }
        else if dir.x >= 0.0 { ((origin_voxel.x + 1.0) - origin.x / VOXEL_SIZE) * VOXEL_SIZE }
        else { (origin.x / VOXEL_SIZE - origin_voxel.x) * VOXEL_SIZE },
        if dir.y.abs() < f32::EPSILON { f32::MAX }
        else if dir.y >= 0.0 { ((origin_voxel.y + 1.0) - origin.y / VOXEL_SIZE) * VOXEL_SIZE }
        else { (origin.y / VOXEL_SIZE - origin_voxel.y) * VOXEL_SIZE },
        if dir.z.abs() < f32::EPSILON { f32::MAX }
        else if dir.z >= 0.0 { ((origin_voxel.z + 1.0) - origin.z / VOXEL_SIZE) * VOXEL_SIZE }
        else { (origin.z / VOXEL_SIZE - origin_voxel.z) * VOXEL_SIZE },
    );
    let mut last_normal = IVec3::ZERO;
    let mut dist = 0.0_f32;
    let n = crate::config::CHUNK_SIZE as i32;

    while dist < max_dist {
        let chunk_pos = ChunkPos(
            pos.x.div_euclid(n),
            pos.y.div_euclid(n),
            pos.z.div_euclid(n),
        );
        let lx = pos.x.rem_euclid(n) as u8;
        let ly = pos.y.rem_euclid(n) as u8;
        let lz = pos.z.rem_euclid(n) as u8;
        let local = LocalVoxelPos::new(lx, ly, lz);

        if let Some(chunk) = world.get(chunk_pos) {
            let voxel_id = chunk.get(local);
            if voxel_id != AIR {
                return Some(VoxelHit { chunk: chunk_pos, local, voxel_id, normal: last_normal });
            }
        }

        if t_max.x < t_max.y && t_max.x < t_max.z {
            dist = t_max.x;
            t_max.x += delta.x;
            pos.x += step.x;
            last_normal = IVec3::new(-step.x, 0, 0);
        } else if t_max.y < t_max.z {
            dist = t_max.y;
            t_max.y += delta.y;
            pos.y += step.y;
            last_normal = IVec3::new(0, -step.y, 0);
        } else {
            dist = t_max.z;
            t_max.z += delta.z;
            pos.z += step.z;
            last_normal = IVec3::new(0, 0, -step.z);
        }
    }
    None
}

#[derive(Resource, Default)]
pub struct TargetedVoxel(pub Option<VoxelHit>);

pub fn update_targeted_voxel(
    camera_query: Query<&Transform, With<crate::player::camera::PlayerCamera>>,
    world: Res<ChunkedWorld>,
    mut targeted: ResMut<TargetedVoxel>,
) {
    let Ok(cam) = camera_query.single() else {
        targeted.0 = None;
        return;
    };
    targeted.0 = raycast_voxels(&world, cam.translation, cam.forward().into(), REACH);
}

use crate::simulation::debris::spawn_debris;
use crate::inventory::Inventory;
use crate::game_mode::GameMode;
use crate::config::CHUNK_SIZE;

pub fn handle_break_place(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    targeted: Res<TargetedVoxel>,
    mut world: ResMut<ChunkedWorld>,
    mut inventory: ResMut<Inventory>,
    game_mode: Res<GameMode>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(hit) = &targeted.0 {
            let tool = inventory.active_tool;
            if !tool.can_break(hit.voxel_id) { return; }

            if let Some(chunk) = world.get_mut(hit.chunk) {
                let voxel_id = chunk.get(hit.local);
                chunk.set(hit.local, crate::types::AIR);
                let n = CHUNK_SIZE as i32;
                let voxel_world = Vec3::new(
                    hit.chunk.0 as f32 * n as f32 * VOXEL_SIZE,
                    hit.chunk.1 as f32 * n as f32 * VOXEL_SIZE,
                    hit.chunk.2 as f32 * n as f32 * VOXEL_SIZE,
                ) + Vec3::new(
                    hit.local.x as f32 * VOXEL_SIZE,
                    hit.local.y as f32 * VOXEL_SIZE,
                    hit.local.z as f32 * VOXEL_SIZE,
                ) + Vec3::splat(VOXEL_SIZE * 0.5);

                for i in 0..tool.debris_count() {
                    let spread = Vec3::new(
                        (i as f32 * 0.3) - 0.3,
                        2.0 + i as f32 * 0.2,
                        (i as f32 * 0.2) - 0.2,
                    );
                    spawn_debris(&mut commands, voxel_id, voxel_world, spread);
                }
            }
        }
    }

    if mouse.just_pressed(MouseButton::Right) {
        if let Some(hit) = &targeted.0 {
            let place_voxel_id = inventory.active_voxel_id();
            if place_voxel_id == crate::types::AIR { return; }

            let n = CHUNK_SIZE as i32;
            let voxel_ivec = IVec3::new(
                hit.chunk.0 * n + hit.local.x as i32,
                hit.chunk.1 * n + hit.local.y as i32,
                hit.chunk.2 * n + hit.local.z as i32,
            ) + hit.normal;

            let place_chunk = ChunkPos(
                voxel_ivec.x.div_euclid(n),
                voxel_ivec.y.div_euclid(n),
                voxel_ivec.z.div_euclid(n),
            );
            let place_local = LocalVoxelPos::new(
                voxel_ivec.x.rem_euclid(n) as u8,
                voxel_ivec.y.rem_euclid(n) as u8,
                voxel_ivec.z.rem_euclid(n) as u8,
            );

            if let Some(chunk) = world.get_mut(place_chunk) {
                let creative = *game_mode == GameMode::Creative;
                let active_slot = inventory.active_slot;
                let can_place = creative || inventory.remove(active_slot, 1);
                if can_place {
                    chunk.set(place_local, place_voxel_id);
                }
            }
        }
    }
}

pub fn handle_pickup(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, With<crate::player::camera::PlayerCamera>>,
    debris_query: Query<(Entity, &Transform, &crate::simulation::debris::DebrisParticle)>,
    mut inventory: ResMut<Inventory>,
) {
    if !keys.just_pressed(KeyCode::KeyF) { return; }
    let Ok(cam) = camera_query.single() else { return };

    for (entity, transform, debris) in &debris_query {
        if transform.translation.distance(cam.translation) <= REACH {
            inventory.add(debris.voxel_id, 1);
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::loading::ChunkedWorld;
    use crate::chunk::Chunk;
    use crate::types::STONE;

    fn world_with_stone_at_voxel(vx: i32, vy: i32, vz: i32) -> ChunkedWorld {
        let n = crate::config::CHUNK_SIZE as i32;
        let chunk_pos = ChunkPos(vx.div_euclid(n), vy.div_euclid(n), vz.div_euclid(n));
        let local = LocalVoxelPos::new(
            vx.rem_euclid(n) as u8,
            vy.rem_euclid(n) as u8,
            vz.rem_euclid(n) as u8,
        );
        let mut chunk = Chunk::new();
        chunk.set(local, STONE);
        let mut w = ChunkedWorld::default();
        w.chunks.insert(chunk_pos, chunk);
        w
    }

    #[test]
    fn ray_hits_stone_directly_ahead() {
        let world = world_with_stone_at_voxel(5, 0, 0);
        let origin = Vec3::new(0.05, 0.05, 0.05);
        let hit = raycast_voxels(&world, origin, Vec3::X, REACH);
        assert!(hit.is_some(), "should hit the stone");
        let hit = hit.unwrap();
        assert_eq!(hit.voxel_id, STONE);
    }

    #[test]
    fn ray_misses_when_nothing_in_path() {
        let world = ChunkedWorld::default();
        let hit = raycast_voxels(&world, Vec3::ZERO, Vec3::X, REACH);
        assert!(hit.is_none());
    }

    #[test]
    fn ray_beyond_reach_does_not_hit() {
        let world = world_with_stone_at_voxel(100, 0, 0);
        let origin = Vec3::new(0.05, 0.05, 0.05);
        let hit = raycast_voxels(&world, origin, Vec3::X, REACH);
        assert!(hit.is_none());
    }
}
