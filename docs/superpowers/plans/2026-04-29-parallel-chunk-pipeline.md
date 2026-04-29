# Parallel Chunk Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move chunk generation and mesh building off the main Bevy thread using `AsyncComputeTaskPool`, eliminating frame stutter and preventing the player from walking into void.

**Architecture:** Two parallel pipelines — generation and meshing — each split into a spawn system (fires background tasks) and a collect system (polls results and applies them). All heavy CPU work runs on background threads; the main thread only manages queues and applies completed results.

**Tech Stack:** Bevy 0.16, `bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future}`, avian3d 0.3.

---

## File map

| File | Role after this plan |
|------|---------------------|
| `voxel_game/src/chunk/meshing.rs` | Add `MeshData` struct + `mesh_data_to_mesh`; `greedy_mesh` returns `MeshData` |
| `voxel_game/src/chunk/rendering.rs` | Replace `remesh_dirty_chunks` with `spawn_meshing_tasks` + `collect_meshed_chunks`; add `MeshingChunks` resource |
| `voxel_game/src/chunk/loading.rs` | Replace sync generation with `PendingGeneration` resource, `GeneratingChunks` resource, `spawn_generation_tasks`, `collect_generated_chunks` |
| `voxel_game/src/chunk/mod.rs` | Register new resources; update system schedule |
| `voxel_game/src/world/procedural.rs` | Move noise instances to struct fields (free per-call speedup) |
| `voxel_game/src/world/mod.rs` | `Box<dyn WorldGenerator>` → `Arc<dyn WorldGenerator>` |
| `voxel_game/src/lib.rs` | Wrap generator in `Arc::new(...)` |
| `voxel_game/tests/parallel_generation.rs` | Integration test: async pipeline fills ChunkedWorld |

---

### Task 1: MeshData — thread-safe mesh output

**Files:**
- Modify: `voxel_game/src/chunk/meshing.rs`
- Modify: `voxel_game/src/chunk/rendering.rs` (call-site update only; full rewrite in Task 6)

- [ ] **Step 1: Write failing tests in meshing.rs**

Replace the test helpers that currently take `&Mesh` with ones that take `&MeshData`. The tests themselves stay identical — only the helpers and the call to `greedy_mesh` change signature.

```rust
// In the #[cfg(test)] mod at the bottom of chunk/meshing.rs
// Replace:
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
// With:
fn vertex_count(data: &MeshData) -> usize { data.positions.len() }
fn index_count(data: &MeshData) -> usize { data.indices.len() }
```

The four test bodies remain exactly the same except calling the updated helpers. Run:

```
cd voxel_game && cargo test chunk::meshing 2>&1 | grep -E "error|FAILED|ok"
```

Expected: compile errors because `MeshData` doesn't exist yet and `greedy_mesh` still returns `Mesh`.

- [ ] **Step 2: Add `MeshData`, update `greedy_mesh`, add `mesh_data_to_mesh`**

In `voxel_game/src/chunk/meshing.rs`, make these changes:

Add `MeshData` after the imports (no new imports needed):

```rust
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals:   Vec<[f32; 3]>,
    pub uvs:       Vec<[f32; 2]>,
    pub indices:   Vec<u32>,
}
```

Change `greedy_mesh` return type from `Mesh` to `MeshData`, and replace its last block:

```rust
// Remove these lines at the end of greedy_mesh:
//   let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
//   mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
//   mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
//   mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
//   mesh.insert_indices(Indices::U32(tri_indices));
//   mesh
// Replace with:
    MeshData { positions, normals, uvs, indices: tri_indices }
```

Add `mesh_data_to_mesh` after `greedy_mesh`:

```rust
pub fn mesh_data_to_mesh(data: &MeshData) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs.clone());
    mesh.insert_indices(Indices::U32(data.indices.clone()));
    mesh
}
```

The `use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology}` and `use bevy::render::render_asset::RenderAssetUsages` imports stay — they are still needed by `mesh_data_to_mesh`.

- [ ] **Step 3: Update `mesh_to_collider` and call site in `rendering.rs`**

In `voxel_game/src/chunk/rendering.rs`, replace the entire `mesh_to_collider` function and its call site in `remesh_dirty_chunks`:

```rust
// Replace the old mesh_to_collider (which took &Mesh) with:
fn mesh_to_collider(data: &crate::chunk::meshing::MeshData) -> Option<Collider> {
    if data.positions.is_empty() || data.indices.is_empty() {
        return None;
    }
    let vertices: Vec<Vec3> = data.positions.iter()
        .map(|p| Vec3::new(p[0], p[1], p[2]))
        .collect();
    let indices: Vec<[u32; 3]> = data.indices.chunks(3)
        .filter_map(|t| if t.len() == 3 { Some([t[0], t[1], t[2]]) } else { None })
        .collect();
    if vertices.is_empty() || indices.is_empty() { return None; }
    Some(Collider::trimesh(vertices, indices))
}
```

In `remesh_dirty_chunks`, replace the three lines that call `greedy_mesh`, check `count_vertices`, and build collider:

```rust
// Remove:
//   let mesh = greedy_mesh(&chunk.voxels);
//   ...
//   if mesh.count_vertices() == 0 { continue; }
//   let collider = mesh_to_collider(&mesh);
//   let mesh_handle = meshes.add(mesh);
// Replace with:
        let data = crate::chunk::meshing::greedy_mesh(&chunk.voxels);
        if data.positions.is_empty() { continue; }
        let collider = mesh_to_collider(&data);
        let mesh_handle = meshes.add(crate::chunk::meshing::mesh_data_to_mesh(&data));
```

Remove the now-unused import `use bevy::render::mesh::{Indices, VertexAttributeValues};` from `rendering.rs`.

- [ ] **Step 4: Run tests**

```
cd voxel_game && cargo test 2>&1 | tail -20
```

Expected: all tests pass. The four meshing tests verify empty/single/adjacent/full chunk geometry counts.

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/chunk/meshing.rs voxel_game/src/chunk/rendering.rs
git commit -m "refactor(meshing): introduce MeshData; greedy_mesh returns raw data"
```

---

### Task 2: Move noise instances to ProceduralGenerator fields

**Files:**
- Modify: `voxel_game/src/world/procedural.rs`

- [ ] **Step 1: Verify existing tests pass as baseline**

```
cd voxel_game && cargo test world::procedural 2>&1 | tail -10
```

Expected: 3 tests pass (high_chunk_is_mostly_air, deep_chunk_is_mostly_solid, surface_chunk_has_mixed_content).

- [ ] **Step 2: Move noise fields into the struct**

Replace the entire contents of `voxel_game/src/world/procedural.rs` with:

```rust
use noise::{NoiseFn, Fbm, Perlin, MultiFractal};
use crate::chunk::Chunk;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{ChunkPos, LocalVoxelPos, AIR, STONE, DIRT, TOPSOIL};
use super::WorldGenerator;

pub struct ProceduralGenerator {
    pub seed: u32,
    pub surface_amplitude: f64,
    pub surface_base_y: f64,
    pub cave_threshold: f64,
    surface_noise: Fbm<Perlin>,
    cave_noise: Perlin,
}

impl ProceduralGenerator {
    pub fn new(seed: u32) -> Self {
        let surface_noise: Fbm<Perlin> = Fbm::new(seed)
            .set_octaves(4)
            .set_frequency(0.005);
        let cave_noise = Perlin::new(seed.wrapping_add(1));
        Self {
            seed,
            surface_amplitude: 40.0,
            surface_base_y: 0.0,
            cave_threshold: 0.65,
            surface_noise,
            cave_noise,
        }
    }
}

impl WorldGenerator for ProceduralGenerator {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let n = CHUNK_SIZE as i32;
        let mut chunk = Chunk::new();

        for lz in 0..n as u8 {
            for lx in 0..n as u8 {
                let wx = pos.0 * n + lx as i32;
                let wz = pos.2 * n + lz as i32;

                let nx = wx as f64 * VOXEL_SIZE as f64;
                let nz = wz as f64 * VOXEL_SIZE as f64;
                let height_offset = self.surface_noise.get([nx, nz]) * self.surface_amplitude;
                let surface_voxel_y = (self.surface_base_y + height_offset) as i32;

                for ly in 0..n as u8 {
                    let wy = pos.1 * n + ly as i32;

                    let voxel = if wy > surface_voxel_y {
                        AIR
                    } else if wy == surface_voxel_y {
                        TOPSOIL
                    } else if wy > surface_voxel_y - 3 {
                        DIRT
                    } else {
                        let cv = self.cave_noise.get([
                            wx as f64 * VOXEL_SIZE as f64 * 0.5,
                            wy as f64 * VOXEL_SIZE as f64 * 0.5,
                            wz as f64 * VOXEL_SIZE as f64 * 0.5,
                        ]);
                        if cv.abs() > self.cave_threshold { AIR } else { STONE }
                    };

                    chunk.set(LocalVoxelPos::new(lx, ly, lz), voxel);
                }
            }
        }

        chunk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gen() -> ProceduralGenerator { ProceduralGenerator::new(42) }

    #[test]
    fn high_chunk_is_mostly_air() {
        let g = make_gen();
        let chunk = g.generate_chunk(ChunkPos(0, 20, 0));
        let air_count = chunk.voxels.iter().filter(|&&v| v == AIR).count();
        let total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
        assert!(air_count > total * 9 / 10, "high chunk should be >90% air, got {air_count}/{total}");
    }

    #[test]
    fn deep_chunk_is_mostly_solid() {
        let g = make_gen();
        let chunk = g.generate_chunk(ChunkPos(0, -10, 0));
        let solid_count = chunk.voxels.iter().filter(|&&v| v != AIR).count();
        let total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
        assert!(solid_count > total / 2, "deep chunk should be >50% solid, got {solid_count}/{total}");
    }

    #[test]
    fn surface_chunk_has_mixed_content() {
        let g = make_gen();
        let chunk = g.generate_chunk(ChunkPos(0, 0, 0));
        let has_air = chunk.voxels.iter().any(|&v| v == AIR);
        let has_solid = chunk.voxels.iter().any(|&v| v != AIR);
        assert!(has_air && has_solid, "surface chunk should have mixed air and solid");
    }
}
```

- [ ] **Step 3: Run tests**

```
cd voxel_game && cargo test 2>&1 | tail -20
```

Expected: all tests pass (same 3 procedural tests plus all others).

- [ ] **Step 4: Commit**

```bash
git add voxel_game/src/world/procedural.rs
git commit -m "perf(world): build noise instances once in ProceduralGenerator::new"
```

---

### Task 3: ActiveWorldGenerator Box → Arc

**Files:**
- Modify: `voxel_game/src/world/mod.rs`
- Modify: `voxel_game/src/lib.rs`

- [ ] **Step 1: Update `world/mod.rs`**

```rust
// Replace the entire file with:
pub mod flat;
pub mod procedural;

use bevy::prelude::*;
use std::sync::Arc;
use crate::chunk::Chunk;
use crate::types::ChunkPos;

pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk;
}

#[derive(Resource, Clone)]
pub struct ActiveWorldGenerator(pub Arc<dyn WorldGenerator>);

pub struct WorldPlugin;
impl Plugin for WorldPlugin {
    fn build(&self, _app: &mut App) {}
}
```

- [ ] **Step 2: Update `lib.rs`**

Change line 29–31 from `Box::new(...)` to `Arc::new(...)`:

```rust
// In lib.rs, add at top: use std::sync::Arc;
// Change:
//   .insert_resource(ActiveWorldGenerator(Box::new(
//       ProceduralGenerator::new(12345),
//   )))
// To:
            .insert_resource(ActiveWorldGenerator(Arc::new(
                ProceduralGenerator::new(12345),
            )))
```

The full updated `lib.rs` top section:

```rust
use bevy::prelude::*;
use std::sync::Arc;
use game_mode::GameMode;
use world::{ActiveWorldGenerator, WorldPlugin};
use world::procedural::ProceduralGenerator;
```

- [ ] **Step 3: Run tests**

```
cd voxel_game && cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add voxel_game/src/world/mod.rs voxel_game/src/lib.rs
git commit -m "refactor(world): ActiveWorldGenerator uses Arc for task-safe sharing"
```

---

### Task 4: PendingGeneration resource; strip sync generation from load_unload_chunks

**Files:**
- Modify: `voxel_game/src/chunk/loading.rs`

`pending_load` is currently `Local<VecDeque<ChunkPos>>` — private to `load_unload_chunks`. Moving it to a `Resource` lets `spawn_generation_tasks` pop from it in a separate system.

- [ ] **Step 1: Replace `loading.rs` with the resource-based version**

```rust
use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future};
use crate::chunk::Chunk;
use crate::types::ChunkPos;
use crate::world::ActiveWorldGenerator;

pub const LOAD_RADIUS: i32 = 10;
pub const MAX_INFLIGHT_GENERATION: usize = 32;

#[derive(Resource, Default)]
pub struct PendingGeneration(pub VecDeque<ChunkPos>);

#[derive(Resource, Default)]
pub struct GeneratingChunks(pub HashMap<ChunkPos, Task<Chunk>>);

#[derive(Resource, Default, Debug)]
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
    mut last_chunk: Local<Option<ChunkPos>>,
    mut pending: ResMut<PendingGeneration>,
) {
    let Ok(player_transform) = player_query.single() else { return };
    let player_chunk = ChunkPos::from_world(player_transform.translation);

    if *last_chunk != Some(player_chunk) {
        *last_chunk = Some(player_chunk);

        world.chunks.retain(|pos, _| {
            (pos.0 - player_chunk.0).abs() <= LOAD_RADIUS
                && (pos.1 - player_chunk.1).abs() <= LOAD_RADIUS
                && (pos.2 - player_chunk.2).abs() <= LOAD_RADIUS
        });

        pending.0.clear();
        let r = LOAD_RADIUS;
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    let pos = ChunkPos(
                        player_chunk.0 + dx,
                        player_chunk.1 + dy,
                        player_chunk.2 + dz,
                    );
                    if !world.chunks.contains_key(&pos) {
                        pending.0.push_back(pos);
                    }
                }
            }
        }
        pending.0.make_contiguous().sort_unstable_by_key(|p| {
            let xz = (p.0 - player_chunk.0).abs() + (p.2 - player_chunk.2).abs();
            let dy = p.1 - player_chunk.1;
            let y_cost = if dy < 0 { (-dy) * 4 } else { dy };
            xz + y_cost
        });
    }
}

pub fn spawn_generation_tasks(
    generator: Res<ActiveWorldGenerator>,
    mut pending: ResMut<PendingGeneration>,
    mut generating: ResMut<GeneratingChunks>,
    world: Res<ChunkedWorld>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let capacity = MAX_INFLIGHT_GENERATION.saturating_sub(generating.0.len());
    let mut spawned = 0;
    while spawned < capacity {
        let Some(pos) = pending.0.pop_front() else { break };
        if world.chunks.contains_key(&pos) || generating.0.contains_key(&pos) {
            continue;
        }
        let gen = generator.0.clone();
        let task = task_pool.spawn(async move { gen.generate_chunk(pos) });
        generating.0.insert(pos, task);
        spawned += 1;
    }
}

pub fn collect_generated_chunks(
    mut generating: ResMut<GeneratingChunks>,
    mut world: ResMut<ChunkedWorld>,
) {
    generating.0.retain(|pos, task| {
        match block_on(future::poll_once(task)) {
            Some(chunk) => {
                world.chunks.entry(*pos).or_insert(chunk);
                false
            }
            None => true,
        }
    });
}
```

- [ ] **Step 2: Run tests (expect compile failure)**

```
cd voxel_game && cargo test 2>&1 | grep "error\[" | head -10
```

Expected: compile errors in `chunk/mod.rs` because it still imports `load_unload_chunks` with the old signature and registers the old resources. These are fixed in Task 7.

- [ ] **Step 3: Update mod.rs temporarily so it compiles**

In `chunk/mod.rs`, add the new imports so the crate compiles while Tasks 5 and 6 are in progress:

```rust
// Replace the current use lines at the top of chunk/mod.rs with:
use loading::{ChunkedWorld, PendingGeneration, GeneratingChunks,
              load_unload_chunks, spawn_generation_tasks, collect_generated_chunks};
use rendering::{ChunkEntities, remesh_dirty_chunks};
```

And in `ChunkPlugin::build`, register the new resources (keep existing system registrations unchanged for now):

```rust
        app
            .init_resource::<ChunkedWorld>()
            .init_resource::<ChunkEntities>()
            .init_resource::<PendingGeneration>()
            .init_resource::<GeneratingChunks>()
            .add_systems(Update, load_unload_chunks)
            .add_systems(Update, spawn_generation_tasks.after(load_unload_chunks))
            .add_systems(Update, collect_generated_chunks.after(spawn_generation_tasks))
            .add_systems(Update, remesh_dirty_chunks.after(collect_generated_chunks));
```

- [ ] **Step 4: Run tests**

```
cd voxel_game && cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/chunk/loading.rs voxel_game/src/chunk/mod.rs
git commit -m "feat(chunk): async generation pipeline — spawn_generation_tasks + collect_generated_chunks"
```

---

### Task 5: Async meshing pipeline

**Files:**
- Modify: `voxel_game/src/chunk/rendering.rs`

Replace the single `remesh_dirty_chunks` system with two systems: `spawn_meshing_tasks` (fires background tasks) and `collect_meshed_chunks` (applies completed results to the world).

- [ ] **Step 1: Replace `rendering.rs` entirely**

```rust
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
    if vertices.is_empty() || indices.is_empty() { return None; }
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
            commands.entity(entity).despawn();
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
            commands.entity(old).despawn();
        }
        if data.positions.is_empty() {
            continue;
        }
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
```

Do not run tests yet — `chunk/mod.rs` still imports `remesh_dirty_chunks` which no longer exists. Proceed directly to Task 6, which fixes mod.rs and runs the full test suite.

---

### Task 6: Wire ChunkPlugin — register resources and update system schedule

**Files:**
- Modify: `voxel_game/src/chunk/mod.rs`

- [ ] **Step 1: Replace `chunk/mod.rs`**

```rust
pub mod meshing;
pub mod loading;
pub mod rendering;

use bevy::prelude::*;
use crate::config::CHUNK_SIZE;
use crate::types::{VoxelId, LocalVoxelPos, AIR};
use loading::{ChunkedWorld, PendingGeneration, GeneratingChunks,
              load_unload_chunks, spawn_generation_tasks, collect_generated_chunks};
use rendering::{ChunkEntities, MeshingChunks, spawn_meshing_tasks, collect_meshed_chunks};

pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ChunkedWorld>()
            .init_resource::<ChunkEntities>()
            .init_resource::<PendingGeneration>()
            .init_resource::<GeneratingChunks>()
            .init_resource::<MeshingChunks>()
            .add_systems(Update, (
                load_unload_chunks,
                spawn_generation_tasks.after(load_unload_chunks),
                collect_generated_chunks.after(spawn_generation_tasks),
                spawn_meshing_tasks.after(collect_generated_chunks),
                collect_meshed_chunks.after(spawn_meshing_tasks),
            ));
    }
}

#[derive(Debug)]
pub struct Chunk {
    pub voxels: Box<[VoxelId]>,
    pub dirty: bool,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chunk_is_all_air() {
        let c = Chunk::new();
        assert_eq!(c.get(LocalVoxelPos::new(0, 0, 0)), AIR);
        assert_eq!(c.get(LocalVoxelPos::new(31, 31, 31)), AIR);
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
        assert!(c.dirty);
        c.dirty = false;
        c.set(LocalVoxelPos::new(0, 0, 0), crate::types::STONE);
        assert!(c.dirty);
    }

    #[test]
    fn is_solid_reflects_voxel_content() {
        let mut c = Chunk::new();
        let pos = LocalVoxelPos::new(4, 4, 4);
        assert!(!c.is_solid(pos));
        c.set(pos, crate::types::STONE);
        assert!(c.is_solid(pos));
    }
}
```

- [ ] **Step 2: Run all tests**

```
cd voxel_game && cargo test 2>&1 | tail -25
```

Expected: all tests pass — 0 failures.

- [ ] **Step 3: Commit**

```bash
git add voxel_game/src/chunk/mod.rs voxel_game/src/chunk/rendering.rs
git commit -m "feat(chunk): async meshing pipeline — spawn_meshing_tasks + collect_meshed_chunks"
```

---

### Task 7: Integration test — async pipeline fills ChunkedWorld

**Files:**
- Create: `voxel_game/tests/parallel_generation.rs`

- [ ] **Step 1: Write failing test**

```rust
// voxel_game/tests/parallel_generation.rs
use std::sync::Arc;
use bevy::prelude::*;
use bevy::tasks::TaskPoolPlugin;
use voxel_game::chunk::loading::{
    ChunkedWorld, PendingGeneration, GeneratingChunks,
    spawn_generation_tasks, collect_generated_chunks,
};
use voxel_game::chunk::Chunk;
use voxel_game::types::ChunkPos;
use voxel_game::world::{ActiveWorldGenerator, WorldGenerator};

struct NullGenerator;
impl WorldGenerator for NullGenerator {
    fn generate_chunk(&self, _pos: ChunkPos) -> Chunk { Chunk::new() }
}

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());
    app.init_resource::<ChunkedWorld>();
    app.init_resource::<PendingGeneration>();
    app.init_resource::<GeneratingChunks>();
    app.insert_resource(ActiveWorldGenerator(Arc::new(NullGenerator)));
    app.add_systems(Update, (
        spawn_generation_tasks,
        collect_generated_chunks.after(spawn_generation_tasks),
    ));
    app
}

#[test]
fn all_queued_positions_are_generated() {
    let mut app = make_app();

    {
        let mut pending = app.world_mut().resource_mut::<PendingGeneration>();
        for x in 0..10i32 {
            pending.0.push_back(ChunkPos(x, 0, 0));
        }
    }

    // Run up to 200 frames (with small sleeps to let tasks complete)
    for _ in 0..200 {
        app.update();
        let count = app.world().resource::<ChunkedWorld>().chunks.len();
        if count >= 10 { break; }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let world = app.world().resource::<ChunkedWorld>();
    assert_eq!(world.chunks.len(), 10, "all 10 queued chunks should be generated");
    for x in 0..10i32 {
        assert!(world.chunks.contains_key(&ChunkPos(x, 0, 0)),
            "chunk ({x},0,0) missing");
    }
}

#[test]
fn in_flight_edit_is_not_lost() {
    // If a chunk is marked dirty again while its mesh task is in-flight,
    // the dirty flag stays true after the task completes so it gets re-queued.
    // This test verifies the generation side: a chunk inserted twice is not
    // duplicated (entry().or_insert keeps the first).
    let mut app = make_app();

    {
        let mut pending = app.world_mut().resource_mut::<PendingGeneration>();
        pending.0.push_back(ChunkPos(0, 0, 0));
        pending.0.push_back(ChunkPos(0, 0, 0)); // duplicate
    }

    for _ in 0..200 {
        app.update();
        let count = app.world().resource::<ChunkedWorld>().chunks.len();
        if count >= 1 { break; }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let world = app.world().resource::<ChunkedWorld>();
    assert_eq!(world.chunks.len(), 1, "duplicate positions should not produce duplicate chunks");
}
```

- [ ] **Step 2: Run the tests**

```
cd voxel_game && cargo test --test parallel_generation 2>&1 | tail -15
```

Expected: both tests pass.

- [ ] **Step 3: Run full test suite**

```
cd voxel_game && cargo test 2>&1 | tail -25
```

Expected: all tests pass — 0 failures.

- [ ] **Step 5: Commit**

```bash
git add voxel_game/tests/parallel_generation.rs
git commit -m "test(chunk): integration tests for async generation pipeline"
```
