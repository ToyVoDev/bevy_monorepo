use bevy::prelude::*;
use avian3d::prelude::*;
use std::collections::HashMap;
use crate::chunk::loading::ChunkedWorld;
use crate::chunk::meshing::{greedy_mesh, mesh_data_to_mesh, MeshData};
use crate::types::ChunkPos;

#[derive(Resource, Default)]
pub struct ChunkEntities(pub HashMap<ChunkPos, Entity>);

pub const MAX_MESHES_PER_FRAME: usize = 4;

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

pub fn remesh_dirty_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world: ResMut<ChunkedWorld>,
    mut chunk_entities: ResMut<ChunkEntities>,
    mut shared_material: Local<Option<Handle<StandardMaterial>>>,
) {
    // Cleanup unloaded
    let unloaded: Vec<ChunkPos> = chunk_entities.0
        .keys()
        .filter(|pos| !world.chunks.contains_key(*pos))
        .copied()
        .collect();
    for pos in unloaded {
        if let Some(entity) = chunk_entities.0.remove(&pos) {
            commands.entity(entity).despawn();
        }
    }

    // Chunks that already have a mesh entity went dirty due to a player edit — always
    // remesh them immediately so block placement/removal is never visually delayed.
    let urgent: Vec<ChunkPos> = chunk_entities.0
        .keys()
        .filter(|pos| world.chunks.get(*pos).map_or(false, |c| c.dirty))
        .copied()
        .collect();

    // Newly generated chunks (no entity yet) are capped per frame.
    let new_dirty: Vec<ChunkPos> = world
        .chunks
        .iter()
        .filter(|(p, c)| c.dirty && !chunk_entities.0.contains_key(p))
        .map(|(p, _)| *p)
        .take(MAX_MESHES_PER_FRAME)
        .collect();

    let dirty_positions: Vec<ChunkPos> = urgent.into_iter().chain(new_dirty).collect();

    if dirty_positions.is_empty() {
        return;
    }

    let material_handle = shared_material
        .get_or_insert_with(|| materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.45, 0.4),
            ..default()
        }))
        .clone();

    for pos in dirty_positions {
        let Some(chunk) = world.get_mut(pos) else { continue };
        chunk.dirty = false;
        let data = greedy_mesh(&chunk.voxels);

        if let Some(old) = chunk_entities.0.remove(&pos) {
            commands.entity(old).despawn();
        }

        if data.positions.is_empty() { continue; }
        let collider = mesh_to_collider(&data);
        let mesh_handle = meshes.add(mesh_data_to_mesh(&data));

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
