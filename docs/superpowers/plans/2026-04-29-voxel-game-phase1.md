# Voxel Game Phase 1 — Foundation & Playable Sandbox

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new `voxel_game` workspace crate where the player can walk around a greedy-meshed flat voxel world in first-person, and the meshing benchmark runs across multiple chunk sizes.

**Architecture:** Chunk-based static world (greedy meshed, avian3d colliders) with two compile-time constants (`CHUNK_SIZE`, `VOXEL_SIZE`) controlling spatial scale. A pluggable `WorldGenerator` trait drives chunk generation; `FlatGenerator` and `GameMode::Creative` are the Phase 1 implementations.

**Tech Stack:** Bevy 0.16, avian3d 0.3, criterion 0.5, noise 0.9 (added in Phase 2)

---

## File Map

| File | Responsibility |
|---|---|
| `voxel_game/Cargo.toml` | Crate manifest, deps, bench targets |
| `src/main.rs` | `App` setup, plugin registration |
| `src/lib.rs` | `VoxelGamePlugin` grouping all sub-plugins |
| `src/config.rs` | `CHUNK_SIZE`, `VOXEL_SIZE` constants |
| `src/types.rs` | `VoxelId`, `ChunkPos`, `LocalVoxelPos`, coord conversions |
| `src/game_mode.rs` | `GameMode` resource |
| `src/chunk/mod.rs` | `Chunk` struct and voxel storage |
| `src/chunk/meshing.rs` | Greedy mesh algorithm (no Bevy deps) |
| `src/chunk/loading.rs` | `ChunkedWorld` resource, load/unload system |
| `src/chunk/rendering.rs` | Spawn/swap mesh+collider entities for dirty chunks |
| `src/world/mod.rs` | `WorldGenerator` trait, `ActiveWorldGenerator` resource |
| `src/world/flat.rs` | `FlatGenerator` |
| `src/player/mod.rs` | `PlayerPlugin` |
| `src/player/controller.rs` | avian3d capsule character |
| `src/player/camera.rs` | First-person camera, mouse look |
| `benches/meshing.rs` | criterion benchmark: CHUNK_SIZE sweep |

---

## Task 1: Project Scaffolding

**Files:**
- Create: `voxel_game/Cargo.toml`
- Create: `voxel_game/src/main.rs`
- Create: `voxel_game/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — add `"voxel_game"` to members)

- [ ] **Step 1: Add crate to workspace**

In `/Users/CollinDie/Code/bevy_monorepo/Cargo.toml`, change the members array:

```toml
members = [
  "avian_3d_character",
  "move_box",
  "untitled_game",
  "wave",
  "compute_shader_falling_sand",
  "ProjectSandBevy",
  "my_game",
  "compute_shader_game_of_life",
  "compute_shader_sand",
  "voxel_game",
]
```

- [ ] **Step 2: Create `voxel_game/Cargo.toml`**

```toml
[package]
name = "voxel_game"
version = "0.1.0"
edition = "2024"

[features]
default = ["dev_native"]
dev = [
    "bevy/dynamic_linking",
    "bevy/bevy_dev_tools",
]
dev_native = ["dev", "bevy/file_watcher"]

[dependencies]
bevy = "0.16"
avian3d = { version = "0.3", features = ["3d", "f32", "parry-f32"] }
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "meshing"
harness = false
```

- [ ] **Step 3: Create `voxel_game/src/lib.rs`**

```rust
pub mod chunk;
pub mod config;
pub mod game_mode;
pub mod player;
pub mod types;
pub mod world;

use bevy::prelude::*;

pub struct VoxelGamePlugin;

impl Plugin for VoxelGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            chunk::ChunkPlugin,
            world::WorldPlugin,
            player::PlayerPlugin,
        ));
    }
}
```

- [ ] **Step 4: Create `voxel_game/src/main.rs`**

```rust
use bevy::prelude::*;
use voxel_game::VoxelGamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxel Game".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VoxelGamePlugin)
        .run();
}
```

- [ ] **Step 5: Verify it compiles**

```bash
cd /Users/CollinDie/Code/bevy_monorepo
cargo build -p voxel_game
```

Expected: compile error about missing modules (`chunk`, `world`, `player`) — that's fine, the modules don't exist yet. The crate itself resolves. If you see an unresolved import for `voxel_game` itself that's OK too — fix by running `cargo check -p voxel_game`.

Actually expected at this stage: ERROR about missing `chunk`, `world`, `player` modules from `lib.rs`. That's expected — proceed.

- [ ] **Step 6: Commit**

```bash
git add voxel_game/ Cargo.toml
git commit -m "feat(voxel_game): scaffold new workspace crate"
```

---

## Task 2: Config & Core Types

**Files:**
- Create: `voxel_game/src/config.rs`
- Create: `voxel_game/src/types.rs`
- Create: `voxel_game/src/game_mode.rs`

- [ ] **Step 1: Write failing tests for coordinate conversion**

Create `voxel_game/src/types.rs` with just the test module first:

```rust
// tests live at bottom of file — stub the types for now
use bevy::prelude::*;

pub type VoxelId = u16;
pub const AIR: VoxelId = 0;
pub const STONE: VoxelId = 1;
pub const DIRT: VoxelId = 2;
pub const TOPSOIL: VoxelId = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct ChunkPos(pub i32, pub i32, pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVoxelPos {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl ChunkPos {
    pub fn to_world_origin(self) -> Vec3 { todo!() }
    pub fn from_world(pos: Vec3) -> Self { todo!() }
}

impl LocalVoxelPos {
    pub fn new(x: u8, y: u8, z: u8) -> Self { todo!() }
    pub fn to_index(self) -> usize { todo!() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_voxel_pos_index_roundtrip() {
        // index 0 = (0,0,0), index 1 = (1,0,0), index CHUNK_SIZE = (0,1,0)
        let pos = LocalVoxelPos { x: 3, y: 5, z: 7 };
        let idx = pos.to_index();
        // Reconstruct from index
        use crate::config::CHUNK_SIZE;
        let x = (idx % CHUNK_SIZE) as u8;
        let y = ((idx / CHUNK_SIZE) % CHUNK_SIZE) as u8;
        let z = (idx / (CHUNK_SIZE * CHUNK_SIZE)) as u8;
        assert_eq!((x, y, z), (pos.x, pos.y, pos.z));
    }

    #[test]
    fn chunk_pos_from_world_origin() {
        use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
        let chunk_world = CHUNK_SIZE as f32 * VOXEL_SIZE;
        // A point exactly at the second chunk's origin
        let pos = Vec3::new(chunk_world, 0.0, 0.0);
        assert_eq!(ChunkPos::from_world(pos), ChunkPos(1, 0, 0));
    }

    #[test]
    fn chunk_pos_to_world_origin_roundtrip() {
        let cp = ChunkPos(2, -1, 3);
        let world = cp.to_world_origin();
        assert_eq!(ChunkPos::from_world(world), cp);
    }
}
```

- [ ] **Step 2: Run tests — expect them to fail (todo!())**

```bash
cargo test -p voxel_game types
```

Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Create `voxel_game/src/config.rs`**

```rust
pub const CHUNK_SIZE: usize = 32;
pub const VOXEL_SIZE: f32 = 0.1;
```

- [ ] **Step 4: Implement types**

Replace the stub implementations in `voxel_game/src/types.rs`:

```rust
use bevy::prelude::*;

pub type VoxelId = u16;
pub const AIR: VoxelId = 0;
pub const STONE: VoxelId = 1;
pub const DIRT: VoxelId = 2;
pub const TOPSOIL: VoxelId = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct ChunkPos(pub i32, pub i32, pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVoxelPos {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl ChunkPos {
    pub fn to_world_origin(self) -> Vec3 {
        let s = crate::config::CHUNK_SIZE as f32 * crate::config::VOXEL_SIZE;
        Vec3::new(self.0 as f32 * s, self.1 as f32 * s, self.2 as f32 * s)
    }

    pub fn from_world(pos: Vec3) -> Self {
        let s = crate::config::CHUNK_SIZE as f32 * crate::config::VOXEL_SIZE;
        ChunkPos(
            pos.x.div_euclid(s) as i32,
            pos.y.div_euclid(s) as i32,
            pos.z.div_euclid(s) as i32,
        )
    }
}

impl LocalVoxelPos {
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        debug_assert!((x as usize) < crate::config::CHUNK_SIZE);
        debug_assert!((y as usize) < crate::config::CHUNK_SIZE);
        debug_assert!((z as usize) < crate::config::CHUNK_SIZE);
        Self { x, y, z }
    }

    pub fn to_index(self) -> usize {
        let n = crate::config::CHUNK_SIZE;
        self.x as usize + self.y as usize * n + self.z as usize * n * n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_voxel_pos_index_roundtrip() {
        let pos = LocalVoxelPos { x: 3, y: 5, z: 7 };
        let idx = pos.to_index();
        use crate::config::CHUNK_SIZE;
        let x = (idx % CHUNK_SIZE) as u8;
        let y = ((idx / CHUNK_SIZE) % CHUNK_SIZE) as u8;
        let z = (idx / (CHUNK_SIZE * CHUNK_SIZE)) as u8;
        assert_eq!((x, y, z), (pos.x, pos.y, pos.z));
    }

    #[test]
    fn chunk_pos_from_world_origin() {
        use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
        let chunk_world = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let pos = Vec3::new(chunk_world, 0.0, 0.0);
        assert_eq!(ChunkPos::from_world(pos), ChunkPos(1, 0, 0));
    }

    #[test]
    fn chunk_pos_to_world_origin_roundtrip() {
        let cp = ChunkPos(2, -1, 3);
        let world = cp.to_world_origin();
        assert_eq!(ChunkPos::from_world(world), cp);
    }
}
```

- [ ] **Step 5: Create `voxel_game/src/game_mode.rs`**

```rust
use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Creative,
    Survival,
}
```

- [ ] **Step 6: Run tests — expect pass**

```bash
cargo test -p voxel_game types
```

Expected: all 3 tests PASS

- [ ] **Step 7: Commit**

```bash
git add voxel_game/src/config.rs voxel_game/src/types.rs voxel_game/src/game_mode.rs
git commit -m "feat(voxel_game): core types and compile-time constants"
```

---

## Task 3: Chunk Storage

**Files:**
- Create: `voxel_game/src/chunk/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `voxel_game/src/chunk/mod.rs`:

```rust
pub mod meshing;
pub mod loading;
pub mod rendering;

use bevy::prelude::*;
use crate::config::CHUNK_SIZE;
use crate::types::{VoxelId, LocalVoxelPos, AIR};

pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
    fn build(&self, _app: &mut App) {}
}

pub struct Chunk {
    pub voxels: Box<[VoxelId]>,
    pub dirty: bool,
}

impl Chunk {
    pub fn new() -> Self { todo!() }
    pub fn get(&self, _pos: LocalVoxelPos) -> VoxelId { todo!() }
    pub fn set(&mut self, _pos: LocalVoxelPos, _id: VoxelId) { todo!() }
    pub fn is_solid(&self, pos: LocalVoxelPos) -> bool { self.get(pos) != AIR }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chunk_is_all_air() {
        let c = Chunk::new();
        assert_eq!(c.get(LocalVoxelPos::new(0, 0, 0)), AIR);
        assert_eq!(c.get(LocalVoxelPos::new(15, 15, 15)), AIR);
    }

    #[test]
    fn set_get_voxel() {
        let mut c = Chunk::new();
        let pos = LocalVoxelPos::new(1, 2, 3);
        c.set(pos, crate::types::STONE);
        assert_eq!(c.get(pos), crate::types::STONE);
        assert_eq!(c.get(LocalVoxelPos::new(0, 0, 0)), AIR);
    }

    #[test]
    fn set_marks_dirty() {
        let mut c = Chunk::new();
        assert!(c.dirty);  // new chunk starts dirty so it gets meshed
        c.dirty = false;
        c.set(LocalVoxelPos::new(0, 0, 0), crate::types::STONE);
        assert!(c.dirty);
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test -p voxel_game chunk::tests
```

Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Implement Chunk**

Replace the stub implementations in `voxel_game/src/chunk/mod.rs`:

```rust
impl Chunk {
    pub fn new() -> Self {
        Self {
            voxels: vec![AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE].into_boxed_slice(),
            dirty: true,
        }
    }

    pub fn get(&self, pos: LocalVoxelPos) -> VoxelId {
        self.voxels[pos.to_index()]
    }

    pub fn set(&mut self, pos: LocalVoxelPos, id: VoxelId) {
        self.voxels[pos.to_index()] = id;
        self.dirty = true;
    }

    pub fn is_solid(&self, pos: LocalVoxelPos) -> bool {
        self.get(pos) != AIR
    }
}
```

Also add stub files so the module declaration compiles:

```bash
touch voxel_game/src/chunk/meshing.rs
touch voxel_game/src/chunk/loading.rs
touch voxel_game/src/chunk/rendering.rs
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p voxel_game chunk::tests
```

Expected: all 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/chunk/
git commit -m "feat(voxel_game): chunk voxel storage"
```

---

## Task 4: Greedy Meshing

**Files:**
- Create: `voxel_game/src/chunk/meshing.rs`
- Create: `voxel_game/benches/meshing.rs`

- [ ] **Step 1: Write failing tests**

```rust
// voxel_game/src/chunk/meshing.rs
use bevy::render::mesh::Mesh;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{VoxelId, AIR, STONE};

/// Generates a greedy-merged triangle mesh from a flat voxel array.
/// `voxels` must have length `CHUNK_SIZE³`, indexed as x + y*N + z*N*N.
/// Faces between two solid voxels are culled. Only boundary faces are emitted.
pub fn greedy_mesh(voxels: &[VoxelId]) -> Mesh {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;

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
        // Two voxels share one internal face (front of voxel0 == back of voxel1).
        // Unmerged: 12 faces. Merged: 10 faces.
        // But greedy also merges the two X-faces into one quad: 8 faces total.
        // 8 faces × 4 verts = 32; 8 × 6 indices = 48
        assert_eq!(vertex_count(&mesh), 32);
        assert_eq!(index_count(&mesh), 48);
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
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test -p voxel_game meshing
```

Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Implement greedy_mesh**

Replace the `todo!()` in `voxel_game/src/chunk/meshing.rs` with the full implementation:

```rust
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{VoxelId, AIR};

fn at(voxels: &[VoxelId], x: usize, y: usize, z: usize) -> VoxelId {
    let n = CHUNK_SIZE;
    voxels[x + y * n + z * n * n]
}

pub fn greedy_mesh(voxels: &[VoxelId]) -> Mesh {
    let n = CHUNK_SIZE;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut tri_indices: Vec<u32> = Vec::new();

    // Each axis d has a u-axis and v-axis for the 2D face plane
    let axes: [(usize, usize, usize); 3] = [(0, 1, 2), (1, 0, 2), (2, 0, 1)];

    for (d, u_ax, v_ax) in axes {
        for layer in 0..n {
            // --- Front faces: voxel at `layer` solid, voxel at `layer+1` air ---
            let mut mask = vec![AIR; n * n];
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
            emit_quads(&mask, n, layer + 1, d, u_ax, v_ax, false,
                &mut positions, &mut normals, &mut uvs, &mut tri_indices);

            // --- Back faces: voxel at `layer` solid, voxel at `layer-1` air ---
            let mut mask = vec![AIR; n * n];
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
            emit_quads(&mask, n, layer, d, u_ax, v_ax, true,
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
    let mut done = vec![false; n * n];

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
            // If faces appear black/invisible, swap the two branches.
            if back_face {
                tri_indices.extend_from_slice(&[base, base+1, base+2, base+1, base+3, base+2]);
            } else {
                tri_indices.extend_from_slice(&[base, base+2, base+1, base+2, base+3, base+1]);
            }

            ui += w;
        }
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p voxel_game meshing
```

Expected: all 4 tests PASS. If `two_adjacent_voxels_merge_internal_faces` fails with 40 vertices instead of 32, the X-face greedy merge didn't fire — double-check that the `done` array correctly tracks merged cells.

- [ ] **Step 5: Create benchmark**

Create `voxel_game/benches/meshing.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use voxel_game::types::STONE;

fn bench_greedy_mesh(c: &mut Criterion) {
    let mut group = c.benchmark_group("greedy_mesh");

    // Rebuild the mesher with different chunk sizes via a closure
    // Note: CHUNK_SIZE is a compile-time constant, so this benchmark
    // measures the *current* CHUNK_SIZE. Change the constant and re-run
    // to compare sizes.
    let n = voxel_game::config::CHUNK_SIZE;

    for fill in [0.1f32, 0.5, 1.0] {
        let voxels: Vec<u16> = (0..n * n * n)
            .map(|i| {
                // Deterministic fill: checkerboard-ish based on position
                let x = i % n;
                let y = (i / n) % n;
                let z = i / (n * n);
                if (x as f32 / n as f32) < fill && (y + z) % 3 != 0 {
                    STONE
                } else {
                    0
                }
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new(format!("CHUNK_SIZE={n}"), format!("fill={fill:.0}")),
            &voxels,
            |b, v| b.iter(|| voxel_game::chunk::meshing::greedy_mesh(black_box(v))),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_greedy_mesh);
criterion_main!(benches);
```

- [ ] **Step 6: Run benchmark to verify it compiles and runs**

```bash
cargo bench -p voxel_game --bench meshing -- --test
```

Expected: output shows "test meshing::bench_greedy_mesh ... ok" (runs once in test mode, not full benchmark)

- [ ] **Step 7: Commit**

```bash
git add voxel_game/src/chunk/meshing.rs voxel_game/benches/
git commit -m "feat(voxel_game): greedy meshing algorithm and benchmark"
```

---

## Task 5: WorldGenerator Trait + FlatGenerator

**Files:**
- Create: `voxel_game/src/world/mod.rs`
- Create: `voxel_game/src/world/flat.rs`

- [ ] **Step 1: Write failing tests**

Create `voxel_game/src/world/mod.rs`:

```rust
pub mod flat;

use bevy::prelude::*;
use crate::chunk::Chunk;
use crate::types::ChunkPos;

pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk;
}

#[derive(Resource)]
pub struct ActiveWorldGenerator(pub Box<dyn WorldGenerator>);

pub struct WorldPlugin;
impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::game_mode::GameMode>();
    }
}
```

Create `voxel_game/src/world/flat.rs` with tests:

```rust
use crate::chunk::Chunk;
use crate::config::CHUNK_SIZE;
use crate::types::{ChunkPos, LocalVoxelPos, VoxelId, AIR};
use super::WorldGenerator;

pub struct FlatGenerator {
    pub surface_y_voxels: i32, // world voxel Y of the top surface
    pub fill_material: VoxelId,
}

impl FlatGenerator {
    pub fn new(surface_y_voxels: i32, fill_material: VoxelId) -> Self {
        Self { surface_y_voxels, fill_material }
    }
}

impl WorldGenerator for FlatGenerator {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk { todo!() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::STONE;

    fn gen() -> FlatGenerator {
        // Surface at voxel Y = 0: chunks at chunk_y < 0 are solid, chunk_y >= 0 are air
        FlatGenerator::new(0, STONE)
    }

    #[test]
    fn chunk_below_surface_is_solid() {
        let g = gen();
        let chunk = g.generate_chunk(ChunkPos(0, -1, 0));
        let pos = LocalVoxelPos::new(0, 0, 0);
        assert_eq!(chunk.get(pos), STONE, "chunk below surface should be solid");
    }

    #[test]
    fn chunk_above_surface_is_air() {
        let g = gen();
        let chunk = g.generate_chunk(ChunkPos(0, 1, 0));
        let pos = LocalVoxelPos::new(0, 0, 0);
        assert_eq!(chunk.get(pos), AIR, "chunk above surface should be air");
    }

    #[test]
    fn surface_chunk_has_mixed_content() {
        // surface_y_voxels = 0 means y=0 is the first air row.
        // Chunk at chunk_y=0 spans world voxel y in [0, CHUNK_SIZE).
        // All voxels in this chunk are above or at the surface — all air.
        let g = gen();
        let chunk = g.generate_chunk(ChunkPos(0, 0, 0));
        // voxel y=0 in chunk 0 is world voxel y=0, which is AIR (surface_y_voxels=0 means surface is below chunk 0)
        assert_eq!(chunk.get(LocalVoxelPos::new(0, 0, 0)), AIR);
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test -p voxel_game flat
```

Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Implement FlatGenerator**

Replace the `todo!()` in `voxel_game/src/world/flat.rs`:

```rust
impl WorldGenerator for FlatGenerator {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let n = CHUNK_SIZE as i32;
        let mut chunk = Chunk::new();
        for z in 0..n as u8 {
            for x in 0..n as u8 {
                for y in 0..n as u8 {
                    let world_y = pos.1 * n + y as i32;
                    let voxel = if world_y < self.surface_y_voxels {
                        self.fill_material
                    } else {
                        AIR
                    };
                    chunk.set(LocalVoxelPos::new(x, y, z), voxel);
                }
            }
        }
        chunk
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p voxel_game flat
```

Expected: all 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/world/
git commit -m "feat(voxel_game): WorldGenerator trait and FlatGenerator"
```

---

## Task 6: Chunk Loading System

**Files:**
- Create: `voxel_game/src/chunk/loading.rs`

- [ ] **Step 1: Write the loading module**

Create `voxel_game/src/chunk/loading.rs`:

```rust
use bevy::prelude::*;
use std::collections::HashMap;
use crate::chunk::Chunk;
use crate::config::CHUNK_SIZE;
use crate::types::ChunkPos;
use crate::world::ActiveWorldGenerator;

pub const LOAD_RADIUS: i32 = 10;

#[derive(Resource, Default)]
pub struct ChunkedWorld {
    pub chunks: HashMap<ChunkPos, Chunk>,
}

impl ChunkedWorld {
    pub fn get(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    pub fn get_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        self.chunks.get_mut(&pos)
    }
}

pub fn load_unload_chunks(
    player_query: Query<&Transform, With<crate::player::Player>>,
    mut world: ResMut<ChunkedWorld>,
    generator: Res<ActiveWorldGenerator>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let player_chunk = ChunkPos::from_world(player_transform.translation);

    // Collect which chunks should be active
    let mut desired = std::collections::HashSet::new();
    let r = LOAD_RADIUS;
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                desired.insert(ChunkPos(
                    player_chunk.0 + dx,
                    player_chunk.1 + dy,
                    player_chunk.2 + dz,
                ));
            }
        }
    }

    // Generate missing chunks
    for &pos in &desired {
        world.chunks.entry(pos).or_insert_with(|| generator.0.generate_chunk(pos));
    }

    // Unload distant chunks (no persistence yet — just drop)
    world.chunks.retain(|pos, _| desired.contains(pos));
}
```

- [ ] **Step 2: Register the system and resource**

Update `voxel_game/src/chunk/mod.rs` — replace the empty `ChunkPlugin::build`:

```rust
use bevy::prelude::*;
use crate::chunk::loading::{ChunkedWorld, load_unload_chunks};

pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ChunkedWorld>()
            .add_systems(Update, (
                load_unload_chunks,
                crate::chunk::rendering::remesh_dirty_chunks,
            ));
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add voxel_game/src/chunk/loading.rs voxel_game/src/chunk/mod.rs
git commit -m "feat(voxel_game): chunk load/unload system"
```

---

## Task 7: Chunk Rendering

**Files:**
- Create: `voxel_game/src/chunk/rendering.rs`

- [ ] **Step 1: Create rendering system**

Create `voxel_game/src/chunk/rendering.rs`:

```rust
use bevy::prelude::*;
use avian3d::prelude::*;
use std::collections::HashMap;
use crate::chunk::loading::ChunkedWorld;
use crate::chunk::meshing::greedy_mesh;
use crate::types::ChunkPos;

#[derive(Resource, Default)]
pub struct ChunkEntities(pub HashMap<ChunkPos, Entity>);

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

    for pos in dirty_positions {
        let Some(chunk) = world.get_mut(pos) else { continue };
        chunk.dirty = false;
        let mesh = greedy_mesh(&chunk.voxels);

        // Despawn old entity if any
        if let Some(old) = chunk_entities.0.remove(&pos) {
            commands.entity(old).despawn_recursive();
        }

        // Skip empty chunks
        if mesh.count_vertices() == 0 {
            continue;
        }

        let mesh_handle = meshes.add(mesh);
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.45, 0.4),
            ..default()
        });

        let entity = commands.spawn((
            PbrBundle {
                mesh: mesh_handle,
                material,
                transform: Transform::from_translation(pos.to_world_origin()),
                ..default()
            },
            RigidBody::Static,
            Collider::trimesh_from_mesh(
                &meshes.get(mesh_handle).unwrap()
            ).unwrap_or_else(|| Collider::cuboid(0.1, 0.1, 0.1)),
            pos,
        )).id();

        chunk_entities.0.insert(pos, entity);
    }
}
```

- [ ] **Step 2: Register `ChunkEntities` resource**

In `voxel_game/src/chunk/mod.rs`, add to `ChunkPlugin::build`:

```rust
app.init_resource::<ChunkEntities>();
```

Import at top of `mod.rs`:
```rust
pub use rendering::ChunkEntities;
```

- [ ] **Step 3: Add ambient light to main.rs**

Update `voxel_game/src/main.rs`:

```rust
use bevy::prelude::*;
use voxel_game::VoxelGamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxel Game".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VoxelGamePlugin)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 500.0,
        })
        .add_systems(Startup, setup_light)
        .run();
}

fn setup_light(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ, -0.5, 0.3, 0.0,
        )),
        ..default()
    });
}
```

- [ ] **Step 4: Commit**

```bash
git add voxel_game/src/chunk/rendering.rs voxel_game/src/main.rs
git commit -m "feat(voxel_game): chunk mesh rendering with avian3d colliders"
```

---

## Task 8: Player Character

**Files:**
- Create: `voxel_game/src/player/mod.rs`
- Create: `voxel_game/src/player/controller.rs`
- Create: `voxel_game/src/player/camera.rs`

- [ ] **Step 1: Create player module**

Create `voxel_game/src/player/mod.rs`:

```rust
pub mod camera;
pub mod controller;

use bevy::prelude::*;
use avian3d::prelude::*;

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PhysicsPlugins::default())
            .add_systems(Startup, controller::spawn_player)
            .add_systems(Update, (
                controller::move_player,
                camera::sync_camera,
            ));
    }
}
```

- [ ] **Step 2: Create controller**

Create `voxel_game/src/player/controller.rs`:

```rust
use bevy::prelude::*;
use avian3d::prelude::*;
use super::Player;

pub fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.9), // radius, half-height
        LockedAxes::ROTATION_LOCKED,
        LinearDamping(0.0),
        Friction::ZERO,
        Restitution::ZERO,
        GravityScale(1.0),
        LinearVelocity::default(),
        TransformBundle::from(Transform::from_xyz(0.5, 5.0, 0.5)),
        Visibility::Hidden, // no visible player mesh yet
    ));
}

pub fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, With<super::camera::PlayerCamera>>,
    mut player_query: Query<(&Transform, &mut LinearVelocity), With<Player>>,
    time: Res<Time>,
) {
    let Ok(cam_transform) = camera_query.get_single() else { return };
    let Ok((player_transform, mut velocity)) = player_query.get_single_mut() else { return };

    let speed = 5.0_f32;
    let jump_impulse = 7.0_f32;

    // Horizontal movement in camera-facing direction
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

    // Jump (simple — no grounded check yet)
    if keys.just_pressed(KeyCode::Space) && velocity.y.abs() < 0.1 {
        velocity.y = jump_impulse;
    }
}
```

- [ ] **Step 3: Create first-person camera**

Create `voxel_game/src/player/camera.rs`:

```rust
use bevy::prelude::*;
use super::Player;

#[derive(Component)]
pub struct PlayerCamera;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        PlayerCamera,
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.8, 0.0), // eye height offset
            ..default()
        },
    ));
}

pub fn sync_camera(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut yaw: Local<f32>,
    mut pitch: Local<f32>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let Ok(mut cam_transform) = camera_query.get_single_mut() else { return };

    // Mouse look
    let sensitivity = 0.002_f32;
    for event in mouse_motion.read() {
        *yaw -= event.delta.x * sensitivity;
        *pitch -= event.delta.y * sensitivity;
        *pitch = pitch.clamp(-1.5, 1.5);
    }

    // Camera follows player + eye height + mouse orientation
    cam_transform.translation = player_transform.translation + Vec3::Y * 0.8;
    cam_transform.rotation = Quat::from_euler(EulerRot::YXZ, *yaw, *pitch, 0.0);
}
```

- [ ] **Step 4: Register spawn_camera and cursor lock in PlayerPlugin**

Update `voxel_game/src/player/mod.rs`:

```rust
pub mod camera;
pub mod controller;

use bevy::prelude::*;
use avian3d::prelude::*;

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PhysicsPlugins::default())
            .add_systems(Startup, (controller::spawn_player, camera::spawn_camera))
            .add_systems(Update, (
                controller::move_player,
                camera::sync_camera,
                cursor_lock,
            ));
    }
}

fn cursor_lock(
    mut windows: Query<&mut Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut window) = windows.get_single_mut() else { return };
    if mouse.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = bevy::window::CursorGrabMode::None;
        window.cursor.visible = true;
    }
}
```

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/player/
git commit -m "feat(voxel_game): first-person player character with avian3d physics"
```

---

## Task 9: Sandbox Default & Playable Milestone

**Files:**
- Modify: `voxel_game/src/lib.rs`
- Modify: `voxel_game/src/world/mod.rs`

- [ ] **Step 1: Wire up default generator and game mode in VoxelGamePlugin**

Update `voxel_game/src/lib.rs`:

```rust
pub mod chunk;
pub mod config;
pub mod game_mode;
pub mod player;
pub mod types;
pub mod world;

use bevy::prelude::*;
use game_mode::GameMode;
use world::{ActiveWorldGenerator, WorldPlugin};
use world::flat::FlatGenerator;
use types::STONE;

pub struct VoxelGamePlugin;

impl Plugin for VoxelGamePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(GameMode::Creative)
            .insert_resource(ActiveWorldGenerator(Box::new(
                FlatGenerator::new(0, STONE),
            )))
            .add_plugins((
                chunk::ChunkPlugin,
                WorldPlugin,
                player::PlayerPlugin,
            ));
    }
}
```

- [ ] **Step 2: Build and run**

```bash
cargo run -p voxel_game
```

Expected: a window opens showing a flat stone world. The player spawns above it and falls onto the surface. WASD to move, mouse to look (click to lock cursor). The world should be visibly greedy-meshed stone chunks.

If chunks appear black: the winding order is wrong. In `meshing.rs`, swap the `tri_indices.extend_from_slice` contents between the `if back_face` and `else` branches and re-run.

If the player falls through the floor: the trimesh collider failed to generate. Check that `Collider::trimesh_from_mesh` is called on the correct mesh handle (the handle must be in `Assets<Mesh>` before access — if not, defer collider generation).

- [ ] **Step 3: Run all tests to confirm nothing regressed**

```bash
cargo test -p voxel_game
```

Expected: all tests pass

- [ ] **Step 4: Run benchmark**

```bash
cargo bench -p voxel_game
```

Expected: HTML report generated at `target/criterion/greedy_mesh/`. Note the timings for `CHUNK_SIZE=32`.

- [ ] **Step 5: Final commit**

```bash
git add -p  # stage any last fixups
git commit -m "feat(voxel_game): playable flat sandbox milestone"
```

---

## Phase 1 Complete

At this point you have:
- A running game with a greedy-meshed flat voxel world
- First-person character movement with avian3d physics
- Benchmark infrastructure for CHUNK_SIZE comparison
- All unit tests passing

**Next:** Phase 2 — Gameplay Mechanics (`docs/superpowers/plans/2026-04-29-voxel-game-phase2.md`)
