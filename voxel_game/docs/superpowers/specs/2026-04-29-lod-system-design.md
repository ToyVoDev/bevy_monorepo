# LOD System Design

## Goal

Render terrain to a 5km horizon using three LOD levels: full-resolution voxel meshes near the player, downsampled super-chunk meshes at mid-range, and heightmap imposters at distance.

## Constants

- `CHUNK_SIZE = 32`, `VOXEL_SIZE = 0.1m` → 1 LOD0 chunk = 3.2m per side
- LOD1 super-chunk: 4×4×4 LOD0 chunks = 12.8m per side, 0.4m voxel resolution
- LOD2 super-chunk: 4×4×4 LOD1 super-chunks = 51.2m per side, 1.6m voxel resolution
- Imposter zone: 2–5km (separate spec, deferred)

## LOD Ring Boundaries

| Level | Inner radius | Outer radius | Unit size |
|-------|-------------|--------------|-----------|
| LOD0  | 0m          | 128m         | 3.2m chunk |
| LOD1  | 128m        | 512m         | 12.8m super-chunk |
| LOD2  | 512m        | 2km          | 51.2m super-chunk |
| Imposters | 2km    | 5km          | deferred |

LOD0 uses the existing chunk pipeline unchanged. LOD1 and LOD2 use the new super-chunk pipeline described below.

Transitions are hard swaps at ring boundaries — no blending or seam solver. When the LOD0 radius expands to cover a region, the corresponding LOD1 super-chunk entity is removed, and vice versa.

## Architecture

Three subsystems, all independent of the existing LOD0 pipeline:

1. **LOD Coordinator** — assigns LOD levels and manages the pending queue
2. **Voxel Downsampler + LOD Mesh Pipeline** — generates super-chunk meshes async
3. **Imposter Renderer** — deferred to a separate spec

## Data Model

```rust
// New types (alongside existing ChunkPos, Chunk, ChunkedWorld)

pub enum LodLevel { Lod1, Lod2 }

#[derive(Hash, Eq, PartialEq, Clone, Copy, Component)]
pub struct SuperChunkPos(pub i32, pub i32, pub i32, pub LodLevel);

impl SuperChunkPos {
    // World-space origin of this super-chunk
    pub fn to_world_origin(&self) -> Vec3;
    // LOD1 voxel size = 0.4m, LOD2 = 1.6m
    pub fn voxel_size(&self) -> f32;
}

pub struct SuperChunk {
    pub voxels: Box<[VoxelId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE]>,
    pub dirty: bool,
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

## LOD Coordinator System

Runs each frame after `load_unload_chunks`.

**Load logic:**
- Compute player LOD0 chunk position
- Enumerate all LOD1 super-chunk positions within 512m (annulus: 128m–512m from player)
- Enumerate all LOD2 super-chunk positions within 2km (annulus: 512m–2km from player)
- For any position not in `SuperChunkedWorld` and not already queued: push to `PendingSuperChunks`

**Unload logic:**
- For each entry in `SuperChunkedWorld` and `SuperChunkEntities`: if outside the appropriate ring, despawn entity and remove from world

**Queue ordering:** Surface-first (same Y-penalty heuristic as `PendingGeneration`).

## Downsampler

Runs on the `AsyncComputeTaskPool`. Takes owned source data (no references):

```rust
// For LOD1: source_chunks is 64 LOD0 voxel arrays, factor = 4
// For LOD2: source_chunks is 64 LOD1 voxel arrays, factor = 4
fn downsample(
    source_chunks: Vec<[VoxelId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE]>,
    factor: usize,
) -> [VoxelId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE]
```

The 64 source arrays are arranged in a 4×4×4 grid. For each output voxel at `(ox, oy, oz)`, reads `factor³` source voxels from the corresponding region and returns the majority non-air type (or `AIR` if all air).

LOD2 is built from LOD1 super-chunk data — never from raw LOD0 chunks. This means LOD1 must be available before LOD2 can be generated (coordinator enforces this: only enqueue LOD2 when the 4×4×4 covering LOD1 super-chunks are all present in `SuperChunkedWorld`).

## LOD Mesh Pipeline

**`spawn_lod_meshing_tasks`:**
- Pops from `PendingSuperChunks`
- Reads the 4×4×4 source chunks from `SuperChunkedWorld` (LOD0 for LOD1 requests, LOD1 for LOD2 requests)
- Skips if any source chunk is missing (re-queues for next frame)
- Spawns `AsyncComputeTaskPool` task: `downsample` → `greedy_mesh(voxels, voxel_size)`
- Stores `Task<MeshData>` in `MeshingLodChunks`
- Shares the `MAX_INFLIGHT_MESHING = 16` cap with the LOD0 meshing pipeline

**`collect_lod_meshed_chunks`:**
- Polls tasks with `block_on(future::poll_once(task))`
- Despawns old entity if present in `SuperChunkEntities`
- Skips if `MeshData.positions` is empty
- Spawns `Mesh3d` + `MeshMaterial3d` entity at `super_chunk_pos.to_world_origin()`
- No `Collider` — super-chunks are visual only
- Uses same shared `StandardMaterial` as LOD0

**`greedy_mesh` change:** Add a `voxel_size: f32` parameter to `greedy_mesh` (and `emit_quads`) replacing the hardcoded `VOXEL_SIZE` constant. LOD0 callers pass `VOXEL_SIZE`; LOD1/LOD2 callers pass `super_chunk_pos.voxel_size()`.

## System Schedule

```rust
// Appended after existing LOD0 systems
.add_systems(Update, (
    lod_coordinator.after(load_unload_chunks),
    spawn_lod_meshing_tasks.after(lod_coordinator),
    collect_lod_meshed_chunks.after(spawn_lod_meshing_tasks),
))
```

## Files

| File | Change |
|------|--------|
| `src/chunk/lod.rs` | New: `LodLevel`, `SuperChunkPos`, `SuperChunk`, `SuperChunkedWorld`, `PendingSuperChunks`, `SuperChunkEntities`, `MeshingLodChunks`, `lod_coordinator`, `spawn_lod_meshing_tasks`, `collect_lod_meshed_chunks`, `downsample` |
| `src/chunk/meshing.rs` | Modify: add `voxel_size: f32` param to `greedy_mesh` and `emit_quads` |
| `src/chunk/mod.rs` | Modify: register new resources, add new systems to schedule |

## Out of Scope

- Imposter heightmap renderer (2–5km) — separate spec
- LOD transition blending / seam stitching
- Physics/colliders for LOD1/LOD2
- Texture atlases (planned separately)
