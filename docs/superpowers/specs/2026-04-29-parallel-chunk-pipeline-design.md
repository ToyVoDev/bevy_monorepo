# Parallel Chunk Pipeline Design

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Move chunk generation and mesh building off the main thread using Bevy's `AsyncComputeTaskPool`, eliminating the frame-rate stutter and ensuring the world loads fast enough that the player never walks into void.

**Architecture:** Two independent async pipelines — generation and meshing — each split into a spawn system and a collect system. The main thread only manages the queue and applies results; all CPU-heavy work runs on background threads.

**Tech Stack:** Bevy 0.16, `bevy::tasks::AsyncComputeTaskPool`, `futures_lite::future::poll_once`, avian3d 0.3.

---

## Generator: fix per-call noise allocation

`ProceduralGenerator::generate_chunk` currently constructs `Fbm<Perlin>` and `Perlin` inside the function on every call. These move to struct fields initialized once in `ProceduralGenerator::new`. Both noise types are `Send + Sync` so they are safe to use from background tasks.

## World generator: Box → Arc

`ActiveWorldGenerator` changes from `Box<dyn WorldGenerator>` to `Arc<dyn WorldGenerator>`. Tasks clone the `Arc` cheaply to share the generator across threads without copying it. The `WorldGenerator` trait bound stays `Send + Sync`.

## MeshData: thread-safe mesh output

`meshing.rs` gains a `MeshData` struct:

```rust
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals:   Vec<[f32; 3]>,
    pub uvs:       Vec<[f32; 2]>,
    pub indices:   Vec<u32>,
}
```

`greedy_mesh` is updated to return `MeshData` instead of `Mesh`. A separate helper `mesh_data_to_mesh(data: &MeshData) -> Mesh` constructs the Bevy `Mesh` from raw data on the main thread. The collider builder (`mesh_to_collider`) is updated to take `&MeshData` directly.

## Generation pipeline (`chunk/loading.rs`)

### Resources

```rust
#[derive(Resource, Default)]
pub struct GeneratingChunks(pub HashMap<ChunkPos, Task<(ChunkPos, Chunk)>>);
```

### Systems

**`load_unload_chunks`** — unchanged except it no longer calls `generate_chunk`. It manages unloading and the `VecDeque` queue only.

**`spawn_generation_tasks`** — runs after `load_unload_chunks`. Each frame:
1. Skip positions already in `GeneratingChunks`.
2. Pop up to `MAX_INFLIGHT_GENERATION` (32) positions from the queue.
3. For each, clone `Arc<dyn WorldGenerator>` and spawn an `AsyncComputeTaskPool` task that calls `generator.generate_chunk(pos)` and returns `(pos, chunk)`.
4. Insert the `Task` into `GeneratingChunks`.

**`collect_generated_chunks`** — runs after `spawn_generation_tasks`. For each entry in `GeneratingChunks`, calls `future::poll_once`. Completed tasks insert the chunk into `ChunkedWorld` and remove the entry from the map.

### Constants

```rust
pub const MAX_INFLIGHT_GENERATION: usize = 32;
```

## Meshing pipeline (`chunk/rendering.rs`)

### Resources

```rust
#[derive(Resource, Default)]
pub struct MeshingChunks(pub HashMap<ChunkPos, Task<(ChunkPos, MeshData)>>);
```

### Systems

**`spawn_meshing_tasks`** — each frame:
1. Clean up entities for unloaded chunks (existing logic, unchanged).
2. For each dirty chunk in `ChunkedWorld` not already in `MeshingChunks`:
   - Clone the voxel slice (`Box<[VoxelId]>` → `Vec<VoxelId>`).
   - Set `chunk.dirty = false` immediately to prevent duplicate task spawns.
   - Spawn a task that calls `greedy_mesh(&voxels)` and returns `(pos, mesh_data)`.
   - Insert into `MeshingChunks`.
3. Cap to `MAX_INFLIGHT_MESHING` (16) new tasks per frame.

**`collect_meshed_chunks`** — polls `MeshingChunks` each frame. For each completed task:
1. Despawn the existing entity for that pos, if any.
2. If `MeshData` is empty (all-air chunk), remove from `ChunkEntities` and continue.
3. Build `Mesh` via `mesh_data_to_mesh`.
4. Build `Collider` via `mesh_to_collider(&mesh_data)`.
5. Add mesh to `Assets<Mesh>`, spawn entity with `Mesh3d`, `MeshMaterial3d`, `Transform`, `RigidBody::Static`, optional `Collider`.
6. Insert into `ChunkEntities`.
7. If the chunk is `dirty` again (player edited while task was in-flight), leave it dirty — it will be re-queued by `spawn_meshing_tasks` next frame.

### Constants

```rust
pub const MAX_INFLIGHT_MESHING: usize = 16;
```

## System ordering (`chunk/mod.rs`)

```rust
app.add_systems(Update, (
    load_unload_chunks,
    spawn_generation_tasks.after(load_unload_chunks),
    collect_generated_chunks.after(spawn_generation_tasks),
    spawn_meshing_tasks.after(collect_generated_chunks),
    collect_meshed_chunks.after(spawn_meshing_tasks),
));
```

Chaining ensures chunks collected from generation this frame can start meshing in the same frame.

## Files changed

| File | Change |
|------|--------|
| `world/procedural.rs` | Move `surface_noise`, `cave_noise` to struct fields |
| `world/mod.rs` | `Box<dyn WorldGenerator>` → `Arc<dyn WorldGenerator>` |
| `chunk/meshing.rs` | Add `MeshData`, update `greedy_mesh` to return `MeshData`, add `mesh_data_to_mesh` helper |
| `chunk/loading.rs` | Add `GeneratingChunks` resource, `spawn_generation_tasks`, `collect_generated_chunks`; remove synchronous generation from `load_unload_chunks` |
| `chunk/rendering.rs` | Add `MeshingChunks` resource, refactor into `spawn_meshing_tasks` + `collect_meshed_chunks`; update collider builder |
| `chunk/mod.rs` | Register new resources, update system schedule |
| `lib.rs` | Update `ActiveWorldGenerator` insert to use `Arc` |

## Testing

- Existing unit tests for `Chunk`, `greedy_mesh`, and `ProceduralGenerator` must continue to pass unchanged.
- New integration test `tests/parallel_generation.rs`: spawn 27 positions around origin, run collect loop until all 27 are in `ChunkedWorld`, assert within a reasonable iteration count.
- New integration test: spawn a dirty chunk task, simulate a player edit mid-flight (set `dirty = true` again before collect), assert the chunk is dirty after collect and gets re-queued.
