use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{VoxelId, AIR};

#[inline(always)]
fn at(voxels: &[VoxelId], x: usize, y: usize, z: usize) -> VoxelId {
    let n = CHUNK_SIZE;
    voxels[x + y * n + z * n * n]
}

/// Generates a greedy-merged triangle mesh from a flat voxel array.
/// `voxels` must have length `CHUNK_SIZE³`, indexed as x + y*N + z*N*N.
/// Faces between two solid voxels are culled. Only boundary faces are emitted.
pub fn greedy_mesh(voxels: &[VoxelId]) -> Mesh {
    let n = CHUNK_SIZE;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut tri_indices: Vec<u32> = Vec::new();

    // Each axis d has a u-axis and v-axis for the 2D face plane
    // (d, u_ax, v_ax) chosen so dv×du = +d for front faces (CCW winding → correct geometric normal).
    // d=X: u=Z, v=Y → Y×Z = X. d=Y: u=X, v=Z → Z×X = Y. d=Z: u=Y, v=X → X×Y = Z.
    let axes: [(usize, usize, usize); 3] = [(0, 2, 1), (1, 0, 2), (2, 1, 0)];

    // Pre-allocate scratch buffers — reused every layer to avoid 384 heap allocs per call
    let mut mask = vec![AIR; n * n];
    let mut done = vec![false; n * n];

    for (d, u_ax, v_ax) in axes {
        for layer in 0..n {
            // --- Front faces: voxel at `layer` solid, voxel at `layer+1` air ---
            mask.fill(AIR);
            for vi in 0..n {
                for ui in 0..n {
                    let mut pos = [0usize; 3];
                    pos[d] = layer;
                    pos[u_ax] = ui;
                    pos[v_ax] = vi;
                    let this = at(voxels, pos[0], pos[1], pos[2]);
                    let next_air = layer + 1 >= n || {
                        let mut np = pos;
                        np[d] = layer + 1;
                        at(voxels, np[0], np[1], np[2]) == AIR
                    };
                    mask[ui + vi * n] = if this != AIR && next_air { this } else { AIR };
                }
            }
            done.fill(false);
            emit_quads(&mask, &mut done, n, layer + 1, d, u_ax, v_ax, false,
                &mut positions, &mut normals, &mut uvs, &mut tri_indices);

            // --- Back faces: voxel at `layer` solid, voxel at `layer-1` air ---
            mask.fill(AIR);
            for vi in 0..n {
                for ui in 0..n {
                    let mut pos = [0usize; 3];
                    pos[d] = layer;
                    pos[u_ax] = ui;
                    pos[v_ax] = vi;
                    let this = at(voxels, pos[0], pos[1], pos[2]);
                    let prev_air = layer == 0 || {
                        let mut pp = pos;
                        pp[d] = layer - 1;
                        at(voxels, pp[0], pp[1], pp[2]) == AIR
                    };
                    mask[ui + vi * n] = if this != AIR && prev_air { this } else { AIR };
                }
            }
            done.fill(false);
            emit_quads(&mask, &mut done, n, layer, d, u_ax, v_ax, true,
                &mut positions, &mut normals, &mut uvs, &mut tri_indices);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(tri_indices));
    mesh
}

fn emit_quads(
    mask: &[VoxelId],
    done: &mut Vec<bool>,
    n: usize,
    layer_coord: usize,
    d: usize,
    u_ax: usize,
    v_ax: usize,
    back_face: bool,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    tri_indices: &mut Vec<u32>,
) {
    let s = VOXEL_SIZE;

    for vi in 0..n {
        let mut ui = 0;
        while ui < n {
            let idx = ui + vi * n;
            let vtype = mask[idx];
            if vtype == AIR || done[idx] {
                ui += 1;
                continue;
            }

            // Width: extend right along u_ax
            let mut w = 1;
            while ui + w < n && mask[(ui + w) + vi * n] == vtype && !done[(ui + w) + vi * n] {
                w += 1;
            }

            // Height: extend up along v_ax
            let mut h = 1;
            'h: while vi + h < n {
                for k in 0..w {
                    let m = (ui + k) + (vi + h) * n;
                    if mask[m] != vtype || done[m] {
                        break 'h;
                    }
                }
                h += 1;
            }

            // Mark used
            for dh in 0..h {
                for dw in 0..w {
                    done[(ui + dw) + (vi + dh) * n] = true;
                }
            }

            // Build the four corners
            let mut origin = [0.0f32; 3];
            origin[d] = layer_coord as f32 * s;
            origin[u_ax] = ui as f32 * s;
            origin[v_ax] = vi as f32 * s;

            let mut du = [0.0f32; 3];
            du[u_ax] = w as f32 * s;
            let mut dv = [0.0f32; 3];
            dv[v_ax] = h as f32 * s;

            let p0 = origin;
            let p1 = [origin[0]+du[0], origin[1]+du[1], origin[2]+du[2]];
            let p2 = [origin[0]+dv[0], origin[1]+dv[1], origin[2]+dv[2]];
            let p3 = [origin[0]+du[0]+dv[0], origin[1]+du[1]+dv[1], origin[2]+du[2]+dv[2]];

            let mut normal = [0.0f32; 3];
            normal[d] = if back_face { -1.0 } else { 1.0 };

            let base = positions.len() as u32;
            positions.extend_from_slice(&[p0, p1, p2, p3]);
            normals.extend_from_slice(&[normal, normal, normal, normal]);
            uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]);

            // CCW winding (Bevy default, right-hand system)
            if back_face {
                tri_indices.extend_from_slice(&[base, base+1, base+2, base+1, base+3, base+2]);
            } else {
                tri_indices.extend_from_slice(&[base, base+2, base+1, base+2, base+3, base+1]);
            }

            ui += w;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;
    use crate::types::STONE;

    fn vertex_count(mesh: &Mesh) -> usize {
        match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(v)) => v.len(),
            _ => 0,
        }
    }

    fn index_count(mesh: &Mesh) -> usize {
        use bevy::render::mesh::Indices;
        match mesh.indices() {
            Some(Indices::U32(i)) => i.len(),
            _ => 0,
        }
    }

    #[test]
    fn empty_chunk_no_geometry() {
        let voxels = vec![AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        let mesh = greedy_mesh(&voxels);
        assert_eq!(vertex_count(&mesh), 0);
        assert_eq!(index_count(&mesh), 0);
    }

    #[test]
    fn single_voxel_has_six_faces() {
        let mut voxels = vec![AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        voxels[0] = STONE; // position (0,0,0)
        let mesh = greedy_mesh(&voxels);
        // 6 faces × 4 vertices = 24 verts; 6 faces × 2 triangles × 3 indices = 36 indices
        assert_eq!(vertex_count(&mesh), 24, "single voxel needs 24 vertices");
        assert_eq!(index_count(&mesh), 36, "single voxel needs 36 indices");
    }

    #[test]
    fn two_adjacent_voxels_merge_internal_faces() {
        let mut voxels = vec![AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        voxels[0] = STONE; // (0,0,0)
        voxels[1] = STONE; // (1,0,0) — adjacent on X axis
        let mesh = greedy_mesh(&voxels);
        // Two voxels share one internal face pair (+X of voxel0 meets -X of voxel1 → culled).
        // Exposed faces: 10 raw quads.
        // Greedy merges coplanar adjacent faces:
        //   +Y (voxel0) + +Y (voxel1) → 1 merged quad
        //   -Y (voxel0) + -Y (voxel1) → 1 merged quad
        //   +Z (voxel0) + +Z (voxel1) → 1 merged quad
        //   -Z (voxel0) + -Z (voxel1) → 1 merged quad
        // Remaining unmerged: -X of voxel0 (1), +X of voxel1 (1) = 2 quads.
        // Total: 4 merged + 2 unmerged = 6 quads.
        // 6 quads × 4 verts = 24; 6 quads × 6 indices = 36
        assert_eq!(vertex_count(&mesh), 24);
        assert_eq!(index_count(&mesh), 36);
    }

    #[test]
    fn full_chunk_only_outer_faces() {
        let voxels = vec![STONE; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        let mesh = greedy_mesh(&voxels);
        // 6 outer faces, each greedy-merged to 1 quad: 6 × 4 = 24 verts, 6 × 6 = 36 indices
        assert_eq!(vertex_count(&mesh), 24);
        assert_eq!(index_count(&mesh), 36);
    }
}
