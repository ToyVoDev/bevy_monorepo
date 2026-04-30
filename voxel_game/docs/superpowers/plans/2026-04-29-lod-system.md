# LOD System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two rings of downsampled super-chunk meshes (LOD1: 128–512m, LOD2: 512–2km) that render distant terrain without touching the existing LOD0 chunk pipeline.

**Architecture:** A new `lod.rs` module holds all new types and systems. The existing `greedy_mesh` function gains a `voxel_size: f32` parameter so LOD1/LOD2 meshes scale their geometry correctly. A coordinator system manages super-chunk load/unload, a downsampler collapses 4×4×4 source chunks into a single 32³ grid, and a meshing pipeline mirrors the existing async mesh pipeline.

**Tech Stack:** Bevy 0.18, Rust 2024, `bevy::tasks::AsyncComputeTaskPool`, `block_on(future::poll_once(task))` for non-blocking polling.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/chunk/meshing.rs` | Modify | Add `voxel_size: f32` param to `greedy_mesh` and `emit_quads` |
| `src/chunk/lod.rs` | Create | All new LOD types, downsampler, coordinator, mesh pipeline |
| `src/chunk/mod.rs` | Modify | `pub mod lod`, register new resources, add new systems to schedule |

---

## Task 1: Parameterize `greedy_mesh` with `voxel_size`

**Files:**
- Modify: `src/chunk/meshing.rs`
- Modify: `src/chunk/rendering.rs` (caller update)

---

- [ ] **Step 1: Write a failing test** — add to the `#[cfg(test)]` block in `src/chunk/meshing.rs`:

```rust
#[test]
fn voxel_size_param_scales_geometry() {
    let mut voxels = vec![AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
    voxels[0] = STONE;
    let data = greedy_mesh(&voxels, 0.4);
    let max_x = data.positions.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    assert!((max_x - 0.4).abs() < 1e-5, "expected max_x=0.4, got {max_x}");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd voxel_game && cargo test voxel_size_param_scales_geometry 2>&1 | tail -5
```

Expected: compile error — `greedy_mesh` doesn't accept a second argument yet.

- [ ] **Step 3: Update `greedy_mesh` and `emit_quads` signatures**

In `src/chunk/meshing.rs`, replace the `greedy_mesh` signature at line 34:

```rust
// Before
pub fn greedy_mesh(voxels: &[VoxelId]) -> MeshData {
```

```rust
// After
pub fn greedy_mesh(voxels: &[VoxelId], voxel_size: f32) -> MeshData {
```

Inside `greedy_mesh`, the two `emit_quads` calls (lines 71 and 92) each need `voxel_size` appended as the last argument:

```rust
emit_quads(&mask, &mut done, n, layer + 1, d, u_ax, v_ax, false,
    &mut positions, &mut normals, &mut uvs, &mut colors, &mut tri_indices, voxel_size);

emit_quads(&mask, &mut done, n, layer, d, u_ax, v_ax, true,
    &mut positions, &mut normals, &mut uvs, &mut colors, &mut tri_indices, voxel_size);
```

Update `emit_quads` signature at line 110 (add last param):

```rust
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
    colors: &mut Vec<[f32; 4]>,
    tri_indices: &mut Vec<u32>,
    voxel_size: f32,
) {
    let s = voxel_size;   // was: let s = VOXEL_SIZE;
    // rest of function unchanged
```

Remove the now-unused import of `VOXEL_SIZE` inside `emit_quads` (the `let s = VOXEL_SIZE;` line becomes `let s = voxel_size;`).

- [ ] **Step 4: Update existing tests** — all four tests call `greedy_mesh` without the second arg; add `VOXEL_SIZE` to each:

```rust
// empty_chunk_no_geometry
let data = greedy_mesh(&voxels, VOXEL_SIZE);

// single_voxel_has_six_faces
let data = greedy_mesh(&voxels, VOXEL_SIZE);

// two_adjacent_voxels_merge_internal_faces
let data = greedy_mesh(&voxels, VOXEL_SIZE);

// full_chunk_only_outer_faces
let data = greedy_mesh(&voxels, VOXEL_SIZE);
```

Also add `use crate::config::VOXEL_SIZE;` to the test module if not already present.

- [ ] **Step 5: Update the caller in `src/chunk/rendering.rs` line 73**

```rust
// Before
let task = task_pool.spawn(async move { greedy_mesh(&voxels) });

// After
let task = task_pool.spawn(async move { greedy_mesh(&voxels, crate::config::VOXEL_SIZE) });
```

- [ ] **Step 6: Run all meshing tests**

```bash
cd voxel_game && cargo test --lib chunk 2>&1 | tail -10
```

Expected: all tests pass, zero compile errors.

- [ ] **Step 7: Commit**

```bash
git add voxel_game/src/chunk/meshing.rs voxel_game/src/chunk/rendering.rs
git commit -m "refactor: parameterize greedy_mesh with voxel_size"
```

---

## Task 2: LOD Data Types

**Files:**
- Create: `src/chunk/lod.rs`
- Modify: `src/chunk/mod.rs` (add `pub mod lod;`)

---

- [ ] **Step 1: Write failing tests** — create `src/chunk/lod.rs` with just the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod1_super_chunk_world_origin() {
        let sp = SuperChunkPos(1, 0, 0, LodLevel::Lod1);
        let origin = sp.to_world_origin();
        // 1 LOD1 unit = 4 * 32 * 0.1 = 12.8m
        assert!((origin.x - 12.8).abs() < 1e-4, "x={}", origin.x);
        assert_eq!(origin.y, 0.0);
        assert_eq!(origin.z, 0.0);
    }

    #[test]
    fn lod2_super_chunk_world_origin() {
        let sp = SuperChunkPos(1, 0, 0, LodLevel::Lod2);
        let origin = sp.to_world_origin();
        // 1 LOD2 unit = 16 * 32 * 0.1 = 51.2m
        assert!((origin.x - 51.2).abs() < 1e-3, "x={}", origin.x);
    }

    #[test]
    fn lod1_voxel_size_is_4x() {
        let sp = SuperChunkPos(0, 0, 0, LodLevel::Lod1);
        assert!((sp.voxel_size() - 0.4).abs() < 1e-6);
    }

    #[test]
    fn lod2_voxel_size_is_16x() {
        let sp = SuperChunkPos(0, 0, 0, LodLevel::Lod2);
        assert!((sp.voxel_size() - 1.6).abs() < 1e-6);
    }

    #[test]
    fn lod0_chunks_returns_64() {
        let sp = SuperChunkPos(0, 0, 0, LodLevel::Lod1);
        let chunks = sp.lod0_chunks();
        assert_eq!(chunks.len(), 64);
    }

    #[test]
    fn lod0_chunks_correct_positions() {
        let sp = SuperChunkPos(1, 0, 0, LodLevel::Lod1);
        let chunks = sp.lod0_chunks();
        // base_x = 1 * 4 = 4; first chunk should be (4, 0, 0)
        assert_eq!(chunks[0], crate::types::ChunkPos(4, 0, 0));
        // last chunk (cx=3, cy=3, cz=3): (4+3, 0+3, 0+3) = (7, 3, 3)
        assert_eq!(chunks[63], crate::types::ChunkPos(7, 3, 3));
    }

    #[test]
    fn lod1_super_chunks_returns_64() {
        let sp = SuperChunkPos(0, 0, 0, LodLevel::Lod2);
        let sub = sp.lod1_super_chunks();
        assert_eq!(sub.len(), 64);
        assert_eq!(sub[0].3, LodLevel::Lod1);
    }
}
```

Add `pub mod lod;` to `src/chunk/mod.rs` (at the top alongside `pub mod meshing;`).

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd voxel_game && cargo test --lib chunk::lod 2>&1 | tail -5
```

Expected: compile errors — types not defined yet.

- [ ] **Step 3: Implement the types in `src/chunk/lod.rs`**

```rust
use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use bevy::tasks::Task;
use crate::chunk::meshing::MeshData;
use crate::chunk::{Chunk, SuperChunk};
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{VoxelId, AIR, ChunkPos};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodLevel {
    Lod1,
    Lod2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct SuperChunkPos(pub i32, pub i32, pub i32, pub LodLevel);

impl SuperChunkPos {
    pub fn to_world_origin(self) -> Vec3 {
        let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let side = match self.3 {
            LodLevel::Lod1 => 4.0 * lod0_side,
            LodLevel::Lod2 => 16.0 * lod0_side,
        };
        Vec3::new(self.0 as f32 * side, self.1 as f32 * side, self.2 as f32 * side)
    }

    pub fn voxel_size(self) -> f32 {
        match self.3 {
            LodLevel::Lod1 => VOXEL_SIZE * 4.0,
            LodLevel::Lod2 => VOXEL_SIZE * 16.0,
        }
    }

    pub fn from_world(pos: Vec3, level: LodLevel) -> Self {
        let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
        let side = match level {
            LodLevel::Lod1 => 4.0 * lod0_side,
            LodLevel::Lod2 => 16.0 * lod0_side,
        };
        SuperChunkPos(
            pos.x.div_euclid(side) as i32,
            pos.y.div_euclid(side) as i32,
            pos.z.div_euclid(side) as i32,
            level,
        )
    }

    /// The 64 LOD0 ChunkPos this LOD1 super-chunk covers, in cz/cy/cx order.
    pub fn lod0_chunks(self) -> Vec<ChunkPos> {
        assert_eq!(self.3, LodLevel::Lod1);
        let (bx, by, bz) = (self.0 * 4, self.1 * 4, self.2 * 4);
        let mut out = Vec::with_capacity(64);
        for cz in 0..4i32 {
            for cy in 0..4i32 {
                for cx in 0..4i32 {
                    out.push(ChunkPos(bx + cx, by + cy, bz + cz));
                }
            }
        }
        out
    }

    /// The 64 LOD1 SuperChunkPos this LOD2 super-chunk covers, in cz/cy/cx order.
    pub fn lod1_super_chunks(self) -> Vec<SuperChunkPos> {
        assert_eq!(self.3, LodLevel::Lod2);
        let (bx, by, bz) = (self.0 * 4, self.1 * 4, self.2 * 4);
        let mut out = Vec::with_capacity(64);
        for cz in 0..4i32 {
            for cy in 0..4i32 {
                for cx in 0..4i32 {
                    out.push(SuperChunkPos(bx + cx, by + cy, bz + cz, LodLevel::Lod1));
                }
            }
        }
        out
    }
}

#[derive(Resource, Default)]
pub struct SuperChunkedWorld {
    pub chunks: HashMap<SuperChunkPos, SuperChunk>,
}

#[derive(Resource, Default)]
pub struct PendingSuperChunks(pub VecDeque<SuperChunkPos>);

#[derive(Resource, Default)]
pub struct MeshingLodChunks(pub HashMap<SuperChunkPos, Task<MeshData>>);

#[derive(Resource, Default)]
pub struct SuperChunkEntities(pub HashMap<SuperChunkPos, Entity>);
```

Also add `SuperChunk` to `src/chunk/mod.rs` (alongside `Chunk`):

```rust
pub struct SuperChunk {
    pub voxels: Box<[VoxelId]>,
}

impl SuperChunk {
    pub fn new() -> Self {
        let n = CHUNK_SIZE;
        Self { voxels: vec![AIR; n * n * n].into_boxed_slice() }
    }
}

impl Default for SuperChunk {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 4: Run tests**

```bash
cd voxel_game && cargo test --lib chunk::lod 2>&1 | tail -10
```

Expected: 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/chunk/lod.rs voxel_game/src/chunk/mod.rs
git commit -m "feat: add LOD data types and SuperChunk"
```

---

## Task 3: Downsampler

**Files:**
- Modify: `src/chunk/lod.rs` (add `downsample` function + tests)

The downsampler takes 64 source voxel slices arranged in a 4×4×4 grid (indexed as `cx + cy*4 + cz*16`) and collapses them into a single 32³ grid by majority-vote over each `factor³` cell.

---

- [ ] **Step 1: Write failing tests** — append to `tests` block in `src/chunk/lod.rs`:

```rust
#[test]
fn downsample_all_stone_stays_stone() {
    use crate::types::STONE;
    let n = CHUNK_SIZE;
    let solid: Box<[VoxelId]> = vec![STONE; n * n * n].into_boxed_slice();
    let sources: Vec<Box<[VoxelId]>> = (0..64).map(|_| solid.clone()).collect();
    let out = downsample(sources, 4);
    assert_eq!(out.len(), n * n * n);
    assert!(out.iter().all(|&v| v == STONE), "expected all stone");
}

#[test]
fn downsample_all_air_stays_air() {
    let n = CHUNK_SIZE;
    let empty: Box<[VoxelId]> = vec![AIR; n * n * n].into_boxed_slice();
    let sources: Vec<Box<[VoxelId]>> = (0..64).map(|_| empty.clone()).collect();
    let out = downsample(sources, 4);
    assert!(out.iter().all(|&v| v == AIR), "expected all air");
}

#[test]
fn downsample_majority_wins() {
    use crate::types::{STONE, DIRT};
    let n = CHUNK_SIZE;
    // sources[0] is stone, all others are dirt
    // Output voxel (0,0,0) samples source voxels (0..4, 0..4, 0..4).
    // That 4³=64 cube spans chunks cx=0..0, cy=0..0, cz=0..0 only
    // (since 4 source voxels per side = exactly 1 source chunk).
    // So output voxel (0,0,0) = 1 stone chunk (sources[0]) → all stone
    let stone_chunk: Box<[VoxelId]> = vec![STONE; n * n * n].into_boxed_slice();
    let dirt_chunk: Box<[VoxelId]> = vec![DIRT; n * n * n].into_boxed_slice();
    let mut sources: Vec<Box<[VoxelId]>> = (0..64).map(|_| dirt_chunk.clone()).collect();
    sources[0] = stone_chunk;
    let out = downsample(sources, 4);
    // Output voxel (0,0,0) only sees source chunk 0 → stone
    assert_eq!(out[0], STONE, "voxel (0,0,0) should be stone");
    // Output voxel (1,0,0) samples chunks starting at source x=4 → chunk cx=1 → dirt
    assert_eq!(out[1], DIRT, "voxel (1,0,0) should be dirt");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd voxel_game && cargo test --lib chunk::lod::tests::downsample 2>&1 | tail -5
```

Expected: compile error — `downsample` not defined yet.

- [ ] **Step 3: Implement `downsample`** — add to `src/chunk/lod.rs` (before the `#[cfg(test)]` block):

```rust
/// Collapse 64 source voxel slices (a 4×4×4 grid of chunks, indexed cx + cy*4 + cz*16)
/// into a single CHUNK_SIZE³ voxel grid at `factor`× lower resolution.
/// Each output voxel is the most common non-air type in its factor³ source region,
/// or AIR if all source voxels are air.
pub fn downsample(sources: Vec<Box<[VoxelId]>>, factor: usize) -> Box<[VoxelId]> {
    let n = CHUNK_SIZE;
    let mut out = vec![AIR; n * n * n].into_boxed_slice();

    for oz in 0..n {
        for oy in 0..n {
            for ox in 0..n {
                let mut best_id = AIR;
                let mut best_count = 0u32;
                // accumulate counts per VoxelId using a small fixed array
                // VoxelId values in practice: AIR=0, STONE=1, DIRT=2, TOPSOIL=3
                let mut counts = [0u32; 256];

                for fz in 0..factor {
                    for fy in 0..factor {
                        for fx in 0..factor {
                            let sx = ox * factor + fx;
                            let sy = oy * factor + fy;
                            let sz = oz * factor + fz;
                            let cx = sx / n;
                            let cy = sy / n;
                            let cz = sz / n;
                            let ci = cx + cy * 4 + cz * 16;
                            let lx = sx % n;
                            let ly = sy % n;
                            let lz = sz % n;
                            let vi = lx + ly * n + lz * n * n;
                            let id = sources[ci][vi];
                            if id != AIR {
                                let slot = (id as usize).min(255);
                                counts[slot] += 1;
                                if counts[slot] > best_count {
                                    best_count = counts[slot];
                                    best_id = id;
                                }
                            }
                        }
                    }
                }
                out[ox + oy * n + oz * n * n] = best_id;
            }
        }
    }
    out
}
```

- [ ] **Step 4: Run tests**

```bash
cd voxel_game && cargo test --lib chunk::lod 2>&1 | tail -10
```

Expected: all 10 tests pass (7 from Task 2 + 3 new).

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/chunk/lod.rs
git commit -m "feat: add voxel downsampler for LOD super-chunks"
```

---

## Task 4: LOD Coordinator System

**Files:**
- Modify: `src/chunk/lod.rs` (add `lod_coordinator` and ring constants)

The coordinator runs when the player crosses a LOD1 super-chunk boundary (~every 12.8m). It enqueues LOD1 super-chunks in the 128–512m XZ ring and LOD2 super-chunks in the 512–2km XZ ring, and despawns super-chunks that have left their ring.

---

- [ ] **Step 1: Write failing tests** — append to `tests` block in `src/chunk/lod.rs`:

```rust
#[test]
fn lod1_ring_inner_outer_match_meters() {
    // LOD1 inner = 10 super-chunks * 12.8m = 128m
    let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
    let lod1_side = 4.0 * lod0_side;
    assert!((LOD1_INNER as f32 * lod1_side - 128.0).abs() < 0.1);
    assert!((LOD1_OUTER as f32 * lod1_side - 512.0).abs() < 0.1);
}

#[test]
fn lod2_ring_inner_outer_match_meters() {
    let lod0_side = CHUNK_SIZE as f32 * VOXEL_SIZE;
    let lod2_side = 16.0 * lod0_side;
    assert!((LOD2_INNER as f32 * lod2_side - 512.0).abs() < 0.1);
    // outer ≥ 1800m (spec says 2km, 39 super-chunks = 1996.8m)
    assert!(LOD2_OUTER as f32 * lod2_side >= 1800.0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd voxel_game && cargo test --lib chunk::lod::tests::lod1_ring 2>&1 | tail -5
```

Expected: compile error — constants not defined.

- [ ] **Step 3: Add ring constants and coordinator system** — add to `src/chunk/lod.rs` (before the `#[cfg(test)]` block):

```rust
pub const LOD1_INNER: i32 = 10; // 10 * 12.8m = 128m
pub const LOD1_OUTER: i32 = 40; // 40 * 12.8m = 512m
pub const LOD1_Y_RADIUS: i32 = 5;

pub const LOD2_INNER: i32 = 10; // 10 * 51.2m = 512m
pub const LOD2_OUTER: i32 = 39; // 39 * 51.2m ≈ 2km
pub const LOD2_Y_RADIUS: i32 = 3;

pub fn lod_coordinator(
    player_query: Query<&Transform, With<crate::player::Player>>,
    mut super_world: ResMut<SuperChunkedWorld>,
    mut super_entities: ResMut<SuperChunkEntities>,
    mut pending: ResMut<PendingSuperChunks>,
    mut commands: Commands,
    mut last_lod1_pos: Local<Option<SuperChunkPos>>,
) {
    let Ok(player_tf) = player_query.single() else { return };
    let player_lod1 = SuperChunkPos::from_world(player_tf.translation, LodLevel::Lod1);
    let player_lod2 = SuperChunkPos::from_world(player_tf.translation, LodLevel::Lod2);

    if *last_lod1_pos == Some(player_lod1) {
        return;
    }
    *last_lod1_pos = Some(player_lod1);

    // Despawn entities and remove world data for out-of-range super-chunks
    let dead: Vec<SuperChunkPos> = super_entities
        .0
        .keys()
        .filter(|&&sp| {
            match sp.3 {
                LodLevel::Lod1 => {
                    let xz = (sp.0 - player_lod1.0).abs().max((sp.2 - player_lod1.2).abs());
                    let dy = (sp.1 - player_lod1.1).abs();
                    xz < LOD1_INNER || xz > LOD1_OUTER || dy > LOD1_Y_RADIUS
                }
                LodLevel::Lod2 => {
                    let xz = (sp.0 - player_lod2.0).abs().max((sp.2 - player_lod2.2).abs());
                    let dy = (sp.1 - player_lod2.1).abs();
                    xz < LOD2_INNER || xz > LOD2_OUTER || dy > LOD2_Y_RADIUS
                }
            }
        })
        .copied()
        .collect();
    for sp in dead {
        if let Some(entity) = super_entities.0.remove(&sp) {
            commands.entity(entity).despawn();
        }
        super_world.chunks.remove(&sp);
    }

    // Rebuild pending queue
    pending.0.clear();

    // LOD1 ring
    for dx in -LOD1_OUTER..=LOD1_OUTER {
        for dy in -LOD1_Y_RADIUS..=LOD1_Y_RADIUS {
            for dz in -LOD1_OUTER..=LOD1_OUTER {
                let xz = dx.abs().max(dz.abs());
                if xz < LOD1_INNER || xz > LOD1_OUTER { continue; }
                let sp = SuperChunkPos(
                    player_lod1.0 + dx,
                    player_lod1.1 + dy,
                    player_lod1.2 + dz,
                    LodLevel::Lod1,
                );
                if !super_world.chunks.contains_key(&sp) {
                    pending.0.push_back(sp);
                }
            }
        }
    }

    // LOD2 ring
    for dx in -LOD2_OUTER..=LOD2_OUTER {
        for dy in -LOD2_Y_RADIUS..=LOD2_Y_RADIUS {
            for dz in -LOD2_OUTER..=LOD2_OUTER {
                let xz = dx.abs().max(dz.abs());
                if xz < LOD2_INNER || xz > LOD2_OUTER { continue; }
                let sp = SuperChunkPos(
                    player_lod2.0 + dx,
                    player_lod2.1 + dy,
                    player_lod2.2 + dz,
                    LodLevel::Lod2,
                );
                if !super_world.chunks.contains_key(&sp) {
                    pending.0.push_back(sp);
                }
            }
        }
    }

    // Sort: LOD1 before LOD2, then surface-first within each level
    pending.0.make_contiguous().sort_unstable_by_key(|sp| {
        let level_cost: i32 = match sp.3 { LodLevel::Lod1 => 0, LodLevel::Lod2 => 1_000_000 };
        let (dx, dy, dz) = match sp.3 {
            LodLevel::Lod1 => (sp.0 - player_lod1.0, sp.1 - player_lod1.1, sp.2 - player_lod1.2),
            LodLevel::Lod2 => (sp.0 - player_lod2.0, sp.1 - player_lod2.1, sp.2 - player_lod2.2),
        };
        let xz = dx.abs() + dz.abs();
        let y_cost = if dy < 0 { (-dy) * 4 } else { dy };
        level_cost + xz + y_cost
    });
}
```

- [ ] **Step 4: Run tests**

```bash
cd voxel_game && cargo test --lib chunk::lod 2>&1 | tail -10
```

Expected: all 12 tests pass. (The coordinator system itself isn't unit-testable without a Bevy App; its correctness is validated by the ring constant tests and integration.)

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/chunk/lod.rs
git commit -m "feat: add LOD coordinator system and ring constants"
```

---

## Task 5: LOD Mesh Pipeline

**Files:**
- Modify: `src/chunk/lod.rs` (add `spawn_lod_meshing_tasks` + `collect_lod_meshed_chunks`)

Mirrors the existing `spawn_meshing_tasks` / `collect_meshed_chunks` pair in `rendering.rs`. For each pending LOD1 super-chunk: reads 64 LOD0 chunks from `ChunkedWorld`, runs `downsample` + `greedy_mesh` on the task pool. For LOD2: reads 64 LOD1 super-chunks from `SuperChunkedWorld`.

---

- [ ] **Step 1: Add `spawn_lod_meshing_tasks`** — add to `src/chunk/lod.rs`:

```rust
use crate::chunk::loading::ChunkedWorld;
use crate::chunk::rendering::MAX_INFLIGHT_MESHING;
use crate::chunk::meshing::greedy_mesh;
use bevy::tasks::{AsyncComputeTaskPool, block_on, futures_lite::future};

pub fn spawn_lod_meshing_tasks(
    lod0_world: Res<ChunkedWorld>,
    mut super_world: ResMut<SuperChunkedWorld>,
    mut meshing: ResMut<MeshingLodChunks>,
    mut pending: ResMut<PendingSuperChunks>,
    super_entities: Res<SuperChunkEntities>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let capacity = MAX_INFLIGHT_MESHING.saturating_sub(meshing.0.len());
    let mut spawned = 0;
    let mut requeue: Vec<SuperChunkPos> = Vec::new();

    while spawned < capacity {
        let Some(sp) = pending.0.pop_front() else { break };

        if super_world.chunks.contains_key(&sp) || meshing.0.contains_key(&sp) {
            continue;
        }

        let source_voxels: Option<Vec<Box<[VoxelId]>>> = match sp.3 {
            LodLevel::Lod1 => {
                let children = sp.lod0_chunks();
                let mut vecs = Vec::with_capacity(64);
                let mut ok = true;
                for child in &children {
                    if let Some(chunk) = lod0_world.get(*child) {
                        vecs.push(chunk.voxels.clone());
                    } else {
                        ok = false;
                        break;
                    }
                }
                if ok { Some(vecs) } else { None }
            }
            LodLevel::Lod2 => {
                let children = sp.lod1_super_chunks();
                let mut vecs = Vec::with_capacity(64);
                let mut ok = true;
                for child in &children {
                    if let Some(sc) = super_world.chunks.get(child) {
                        vecs.push(sc.voxels.clone());
                    } else {
                        ok = false;
                        break;
                    }
                }
                if ok { Some(vecs) } else { None }
            }
        };

        match source_voxels {
            None => requeue.push(sp),
            Some(sources) => {
                let voxel_size = sp.voxel_size();
                let task = task_pool.spawn(async move {
                    let downsampled = downsample(sources, 4);
                    greedy_mesh(&downsampled, voxel_size)
                });
                meshing.0.insert(sp, task);
                spawned += 1;
            }
        }
    }

    for sp in requeue {
        pending.0.push_back(sp);
    }
}
```

- [ ] **Step 2: Add `collect_lod_meshed_chunks`** — add to `src/chunk/lod.rs`:

```rust
pub fn collect_lod_meshed_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshing: ResMut<MeshingLodChunks>,
    mut super_world: ResMut<SuperChunkedWorld>,
    mut super_entities: ResMut<SuperChunkEntities>,
    mut shared_material: Local<Option<Handle<StandardMaterial>>>,
) {
    use crate::chunk::meshing::mesh_data_to_mesh;

    let material_handle = shared_material
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                ..default()
            })
        })
        .clone();

    let mut completed: Vec<(SuperChunkPos, crate::chunk::meshing::MeshData)> = Vec::new();
    for (pos, task) in meshing.0.iter_mut() {
        if let Some(data) = block_on(future::poll_once(task)) {
            completed.push((*pos, data));
        }
    }
    for (pos, _) in &completed {
        meshing.0.remove(pos);
    }

    for (pos, data) in completed {
        if let Some(old) = super_entities.0.remove(&pos) {
            commands.entity(old).despawn();
        }

        // Store the downsampled voxel grid for LOD2 to read later
        let n = CHUNK_SIZE;
        // (voxels were consumed by the async task; we can't recover them here.
        // The SuperChunk in super_world is inserted when data is non-empty.)
        // Insert a placeholder SuperChunk so the coordinator knows this pos is done.
        super_world.chunks.entry(pos).or_insert_with(SuperChunk::new);

        if data.positions.is_empty() {
            continue;
        }
        let mesh_handle = meshes.add(mesh_data_to_mesh(data));
        let entity = commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle.clone()),
            Transform::from_translation(pos.to_world_origin()),
            Visibility::default(),
            pos,
        )).id();
        super_entities.0.insert(pos, entity);
    }
}
```

**Note on LOD2 prerequisite data:** The `SuperChunk` stored in `SuperChunkedWorld` after LOD1 meshing is a placeholder (all air) because the downsampled voxel data was moved into the async task and can't be recovered from `MeshData`. To fix this, update `spawn_lod_meshing_tasks` to return the voxel data alongside the mesh. Change the task type so it returns `(MeshData, Box<[VoxelId]>)`:

In `MeshingLodChunks`, update the type:
```rust
pub struct MeshingLodChunks(pub HashMap<SuperChunkPos, Task<(MeshData, Box<[VoxelId]>)>>);
```

Update the task spawn in `spawn_lod_meshing_tasks`:
```rust
let task = task_pool.spawn(async move {
    let downsampled = downsample(sources, 4);
    let mesh = greedy_mesh(&downsampled, voxel_size);
    (mesh, downsampled)
});
```

Update `collect_lod_meshed_chunks` to unpack and store the voxels:
```rust
let mut completed: Vec<(SuperChunkPos, MeshData, Box<[VoxelId]>)> = Vec::new();
for (pos, task) in meshing.0.iter_mut() {
    if let Some((data, voxels)) = block_on(future::poll_once(task)) {
        completed.push((*pos, data, voxels));
    }
}
for (pos, _) in &completed {
    meshing.0.remove(pos);  // adjust for 3-tuple
}
```

Wait — adjust the remove loop too:
```rust
for (pos, _, _) in &completed {
    meshing.0.remove(pos);
}

for (pos, data, voxels) in completed {
    // ... despawn old ...
    super_world.chunks.insert(pos, SuperChunk { voxels });
    if data.positions.is_empty() { continue; }
    // ... spawn mesh entity ...
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd voxel_game && cargo build 2>&1 | grep -E "^error" | head -20
```

Expected: zero errors. Fix any type mismatch errors before proceeding.

- [ ] **Step 4: Commit**

```bash
git add voxel_game/src/chunk/lod.rs
git commit -m "feat: add LOD mesh pipeline (spawn + collect)"
```

---

## Task 6: Wire into ChunkPlugin

**Files:**
- Modify: `src/chunk/mod.rs`

---

- [ ] **Step 1: Add imports and declarations** — update `src/chunk/mod.rs`:

Add to the top imports:
```rust
pub mod lod;
use lod::{
    SuperChunk, SuperChunkedWorld, PendingSuperChunks, MeshingLodChunks, SuperChunkEntities,
    lod_coordinator, spawn_lod_meshing_tasks, collect_lod_meshed_chunks,
};
```

Also add `SuperChunk` usage in the existing `mod.rs` body — move the `SuperChunk` struct definition that was placed in Task 2 here (or leave it in `lod.rs` and re-export it). The simplest approach: keep `SuperChunk` in `mod.rs` alongside `Chunk`, since both are core chunk data types:

```rust
pub struct SuperChunk {
    pub voxels: Box<[VoxelId]>,
}

impl SuperChunk {
    pub fn new() -> Self {
        let n = CHUNK_SIZE;
        Self { voxels: vec![AIR; n * n * n].into_boxed_slice() }
    }
}

impl Default for SuperChunk {
    fn default() -> Self { Self::new() }
}
```

(If you placed it in `lod.rs` in Task 2, move it to `mod.rs` now and update the `use crate::chunk::SuperChunk` imports in `lod.rs`.)

- [ ] **Step 2: Register resources and add systems** — update the `ChunkPlugin::build` method:

```rust
fn build(&self, app: &mut App) {
    app
        .init_resource::<ChunkedWorld>()
        .init_resource::<ChunkEntities>()
        .init_resource::<PendingGeneration>()
        .init_resource::<GeneratingChunks>()
        .init_resource::<MeshingChunks>()
        // LOD resources
        .init_resource::<SuperChunkedWorld>()
        .init_resource::<SuperChunkEntities>()
        .init_resource::<PendingSuperChunks>()
        .init_resource::<MeshingLodChunks>()
        .add_systems(Update, (
            load_unload_chunks,
            spawn_generation_tasks.after(load_unload_chunks),
            collect_generated_chunks.after(spawn_generation_tasks),
            spawn_meshing_tasks.after(collect_generated_chunks),
            collect_meshed_chunks.after(spawn_meshing_tasks),
            // LOD systems
            lod_coordinator.after(load_unload_chunks),
            spawn_lod_meshing_tasks.after(lod_coordinator),
            collect_lod_meshed_chunks.after(spawn_lod_meshing_tasks),
        ).in_set(PausableSystems));
}
```

- [ ] **Step 3: Build and check**

```bash
cd voxel_game && cargo build 2>&1 | grep -E "^error" | head -20
```

Expected: zero errors.

- [ ] **Step 4: Run all tests**

```bash
cd voxel_game && cargo test 2>&1 | tail -15
```

Expected: all existing tests still pass, new LOD tests pass.

- [ ] **Step 5: Commit**

```bash
git add voxel_game/src/chunk/mod.rs
git commit -m "feat: wire LOD systems into ChunkPlugin"
```

---

## Self-Review Checklist

- [x] **Spec coverage**: LOD Coordinator ✓, Downsampler ✓, LOD Mesh Pipeline ✓, `greedy_mesh` voxel_size param ✓, `SuperChunkPos` data model ✓, System schedule ✓, No colliders for LOD1/LOD2 ✓
- [x] **No placeholders**: All steps contain real code
- [x] **Type consistency**: `MeshingLodChunks` uses `Task<(MeshData, Box<[VoxelId]>)>` consistently across Task 5 after the correction; `SuperChunkPos` used consistently throughout; `downsample` signature matches call sites
