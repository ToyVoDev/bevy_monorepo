use bevy::prelude::*;
use avian3d::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future};
use std::collections::HashMap;
use crate::chunk::loading::ChunkedWorld;
use crate::chunk::meshing::{greedy_mesh, mesh_data_to_mesh, MeshData};
use crate::types::{ChunkPos, VoxelId};

#[derive(Resource, Default)]
pub struct ChunkEntities(pub HashMap<ChunkPos, Entity>);

#[derive(Resource, Default)]
pub struct MeshingChunks(pub HashMap<ChunkPos, Task<MeshData>>);

pub const MAX_INFLIGHT_MESHING: usize = 16;

fn mesh_to_collider(data: &MeshData) -> Option<Collider> {
    if data.positions.is_empty() || data.indices.is_empty() {
        return None;
    }
    let vertices: Vec<Vec3> = data.positions.iter()
        .map(|p| Vec3::new(p[0], p[1], p[2]))
        .collect();
    let indices: Vec<[u32; 3]> = data.indices.chunks(3)
        .filter_map(|t| if t.len() == 3 { Some([t[0], t[1], t[2]]) } else { None })
        .collect();
    Some(Collider::trimesh(vertices, indices))
}

pub fn spawn_meshing_tasks(
    mut commands: Commands,
    mut world: ResMut<ChunkedWorld>,
    mut chunk_entities: ResMut<ChunkEntities>,
    mut meshing: ResMut<MeshingChunks>,
) {
    // Despawn entities for chunks that have been unloaded
    let unloaded: Vec<ChunkPos> = chunk_entities.0
        .keys()
        .filter(|pos| !world.chunks.contains_key(*pos))
        .copied()
        .collect();
    for pos in unloaded {
        if let Some(entity) = chunk_entities.0.remove(&pos) {
            commands.entity(entity).despawn_recursive();
        }
    }

    let task_pool = AsyncComputeTaskPool::get();

    // Urgent: already-meshed chunks that went dirty (player edits) — bypass cap
    let urgent: Vec<ChunkPos> = chunk_entities.0
        .keys()
        .filter(|pos| {
            world.chunks.get(*pos).map_or(false, |c| c.dirty)
                && !meshing.0.contains_key(*pos)
        })
        .copied()
        .collect();

    // New dirty chunks (just generated), capped to avoid flooding the task pool
    let capacity = MAX_INFLIGHT_MESHING.saturating_sub(meshing.0.len());
    let new_dirty: Vec<ChunkPos> = world.chunks
        .iter()
        .filter(|(p, c)| c.dirty && !chunk_entities.0.contains_key(p) && !meshing.0.contains_key(p))
        .map(|(p, _)| *p)
        .take(capacity)
        .collect();

    for pos in urgent.into_iter().chain(new_dirty) {
        if let Some(chunk) = world.get_mut(pos) {
            chunk.dirty = false;
            let voxels: Vec<VoxelId> = chunk.voxels.to_vec();
            let task = task_pool.spawn(async move { greedy_mesh(&voxels) });
            meshing.0.insert(pos, task);
        }
    }
}

pub fn collect_meshed_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshing: ResMut<MeshingChunks>,
    mut chunk_entities: ResMut<ChunkEntities>,
    mut shared_material: Local<Option<Handle<StandardMaterial>>>,
) {
    let material_handle = shared_material
        .get_or_insert_with(|| materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.45, 0.4),
            ..default()
        }))
        .clone();

    let mut completed: Vec<(ChunkPos, MeshData)> = Vec::new();
    for (pos, task) in meshing.0.iter_mut() {
        if let Some(data) = block_on(future::poll_once(task)) {
            completed.push((*pos, data));
        }
    }
    for (pos, _) in &completed {
        meshing.0.remove(pos);
    }

    for (pos, data) in completed {
        if let Some(old) = chunk_entities.0.remove(&pos) {
            commands.entity(old).despawn_recursive();
        }
        if data.positions.is_empty() {
            continue;
        }
        let collider = mesh_to_collider(&data);
        let mesh_handle = meshes.add(mesh_data_to_mesh(data));
        let mut entity_cmd = commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle.clone()),
            Transform::from_translation(pos.to_world_origin()),
            Visibility::default(),
            RigidBody::Static,
            pos,
        ));
        if let Some(col) = collider {
            entity_cmd.insert(col);
        }
        chunk_entities.0.insert(pos, entity_cmd.id());
    }
}
