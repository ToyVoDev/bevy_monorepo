use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use avian3d::prelude::*;
use std::collections::HashMap;
use crate::chunk::loading::ChunkedWorld;
use crate::chunk::meshing::greedy_mesh;
use crate::types::ChunkPos;

#[derive(Resource, Default)]
pub struct ChunkEntities(pub HashMap<ChunkPos, Entity>);

fn mesh_to_collider(mesh: &Mesh) -> Option<Collider> {
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION)? {
        VertexAttributeValues::Float32x3(v) => v.clone(),
        _ => return None,
    };
    let vertices: Vec<Vec3> = positions.iter().map(|p| Vec3::new(p[0], p[1], p[2])).collect();

    let indices: Vec<[u32; 3]> = match mesh.indices()? {
        Indices::U32(idx) => idx.chunks(3).filter_map(|t| {
            if t.len() == 3 { Some([t[0], t[1], t[2]]) } else { None }
        }).collect(),
        Indices::U16(idx) => idx.chunks(3).filter_map(|t| {
            if t.len() == 3 { Some([t[0] as u32, t[1] as u32, t[2] as u32]) } else { None }
        }).collect(),
    };

    if vertices.is_empty() || indices.is_empty() {
        return None;
    }
    Some(Collider::trimesh(vertices, indices))
}

pub fn remesh_dirty_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world: ResMut<ChunkedWorld>,
    mut chunk_entities: ResMut<ChunkEntities>,
) {
    let dirty_positions: Vec<ChunkPos> = world
        .chunks
        .iter()
        .filter(|(_, c)| c.dirty)
        .map(|(p, _)| *p)
        .collect();

    if dirty_positions.is_empty() {
        return;
    }

    // One material shared across all chunks remeshed this frame
    let shared_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.45, 0.4),
        ..default()
    });

    for pos in dirty_positions {
        let Some(chunk) = world.get_mut(pos) else { continue };
        chunk.dirty = false;
        let mesh = greedy_mesh(&chunk.voxels);

        if let Some(old) = chunk_entities.0.remove(&pos) {
            commands.entity(old).despawn();
        }

        if mesh.count_vertices() == 0 {
            continue;
        }

        let collider = mesh_to_collider(&mesh);
        let mesh_handle = meshes.add(mesh);
        let material = shared_material.clone();

        let mut entity_cmd = commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material),
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

    // Despawn entities for chunks that have been unloaded
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
}
