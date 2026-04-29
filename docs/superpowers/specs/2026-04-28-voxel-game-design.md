# Voxel Game Design — Decimeter Scale

**Date:** 2026-04-28
**Engine:** Bevy 0.17 (Rust)
**Monorepo crate:** `bevy_monorepo` (new workspace member)

---

## Overview

A survival/exploration game built on decimeter-scale voxels (10cm per voxel). The finer resolution versus Minecraft-scale (1m) allows natural-feeling tunnels, doorways, and terrain features without being constrained by block size. The initial milestone is a sandbox/creative mode that doubles as a development and debugging environment; light survival (resource gathering + crafting) follows.

Core differentiator: dynamic debris physics. Breaking a solid voxel ejects loose debris particles that fall under gravity, pile up, and must be manually picked up — or eventually solidify back into the static world.

---

## Compile-Time Constants

All spatial parameters are controlled by two compile-time constants in a top-level config module:

```rust
pub const CHUNK_SIZE: usize = 32;  // voxels per chunk side
pub const VOXEL_SIZE: f32 = 0.1;   // meters per voxel (decimeter default)
```

Changing either constant and recompiling produces a fully consistent build. These constants are embedded in the `WorldManifest` on save; loading a world with mismatched constants is a hard error. The `CHUNK_SIZE` constant is the primary knob for meshing/generation benchmarks.

---

## Section 1: World Structure & Chunking

The world is a large bounded voxel grid divided into chunks of `CHUNK_SIZE³` voxels. At the default 32³ / 0.1m settings, each chunk is 3.2m per side.

**Coordinates** use a two-level split:
- `ChunkPos(i32, i32, i32)` — chunk grid position
- `LocalVoxelPos(u8, u8, u8)` — voxel position within a chunk (0..CHUNK_SIZE; `u8` holds for `CHUNK_SIZE` ≤ 255, covering all benchmark sizes)

World-to-chunk and chunk-to-world conversions always go through `CHUNK_SIZE` and `VOXEL_SIZE`, never hardcoded numbers.

**World size** is large but bounded (~2km scale). The world wraps at its boundary — `ChunkPos` values are folded modulo the world extent, giving the feel of a finite planet with no hard edge. Wrapping is handled at chunk generation time so noise sampling tiles seamlessly.

**Active chunks** are those within a configurable load radius around the player (~10 chunks). A 10-chunk radius is a 21-chunk-per-side cube (21³ ≈ 9,000 active chunks). At 32KB per chunk (32³ × 2 bytes for `u16` voxel IDs), peak memory for active chunks is ~288MB — within budget.

Chunks are stored in a `HashMap<ChunkPos, Chunk>` resource. Chunks outside the load radius are unloaded; their modified state is flushed to disk first if dirty.

---

## Section 2: Static World Layer

### Voxel Storage

```rust
pub struct Chunk {
    voxels: Box<[VoxelId; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE]>,
    dirty: bool,
}

pub type VoxelId = u16;  // 0 = air, 1..=65535 = material types
```

`VoxelId` of `0` is air — the fast empty check. `u16` gives 65,535 material types, sufficient for a full material library with room to grow.

### Greedy Meshing

When a chunk is marked dirty, a background task on Bevy's `AsyncComputeTaskPool` runs greedy meshing over the chunk's voxel array and produces:
- A `Mesh` (triangle geometry for rendering)
- A `Collider` (for `avian3d` player collision)

The main thread swaps the old mesh/collider for the new ones when the task completes. Adjacent chunks that share a face with the modified voxel are also marked dirty to handle seam stitching.

The meshing algorithm is pure Rust with no Bevy dependencies — fully unit-testable and benchmark-able in isolation.

---

## Section 3: Dynamic Simulation Layer (Debris)

When a voxel is broken it is removed from the static chunk and spawned into the **dynamic layer** as a debris particle:

```rust
pub struct DebrisParticle {
    pub voxel_id: VoxelId,
    pub position: Vec3,
    pub velocity: Vec3,
}
```

The dynamic layer runs a localized cellular automata simulation drawn from `ProjectSandBevy`. Debris falls under gravity and checks for solid surfaces by sampling static chunk voxel data directly — no `avian3d` colliders involved for debris.

### Debris Endpoints

**Picked up:** The player presses F (or interacts) with a debris particle within reach distance. The particle is removed and its `VoxelId` is added to the player's inventory.

**Solidified:** Debris that has been at rest for a configurable duration writes its `VoxelId` back into the static chunk at its current world position, marks the chunk dirty, and removes itself as a particle. The voxel becomes permanent world geometry again. This replaces despawn as the natural endpoint — loose debris fills gaps, sand piles build up into terrain, and the world's topology evolves organically.

### Player / Debris Interaction

The player passes through debris particles by default — no `avian3d` interaction between the player capsule and debris. This avoids needing any CA↔physics bridge and can be revisited if it feels wrong during playtesting.

---

## Section 4: Inventory & Crafting

### Inventory

A fixed-size grid of slots:

```rust
pub struct InventorySlot {
    pub voxel_id: VoxelId,
    pub count: u16,
}
```

Picking up a debris particle increments the matching slot's count. Placing a voxel from inventory writes it into the static chunk (same code path as debris solidification) and decrements the count.

In **Creative** game mode the inventory is effectively infinite — placing voxels does not consume count.

### Crafting

Recipes are defined in a data file (RON) loaded at startup — not hardcoded. A recipe is a list of `(VoxelId, count)` inputs mapping to a `(VoxelId, count)` output:

```rust
pub struct Recipe {
    pub inputs: Vec<(VoxelId, u16)>,
    pub output: (VoxelId, u16),
}
```

The crafting UI shows recipes filtered to those the player can currently fulfill. No crafting grid, no shaped recipes — just ingredient combination.

### Tools

Tools are inventory items with a `ToolType` that determines:
- Which `VoxelId` types they can break
- How many debris particles one swing ejects (e.g., a pickaxe on stone ejects 1–3 `VoxelId::Stone` debris)

No tool durability in the initial scope.

---

## Section 5: World Generation

### WorldGenerator Trait

World generation is pluggable via a trait:

```rust
pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk;
}

#[derive(Resource)]
pub struct ActiveWorldGenerator(pub Box<dyn WorldGenerator>);
```

The chunk loading system calls `generator.0.generate_chunk(pos)` without knowing which strategy is active. New generators implement the trait and are swapped in at startup.

### Initial Implementations

**`ProceduralGenerator`**
Layered generation per chunk:
1. **Heightmap** — 2D Simplex noise (multi-octave) gives surface elevation per voxel column. Cost scales with `CHUNK_SIZE²`.
2. **Biome fill** — columns below surface fill with layered materials (topsoil → stone → deep rock).
3. **Cave carving** — 3D domain-warped Simplex noise subtracts voxels below the surface. At decimeter scale, carved caves feel like real tunnels.
4. **Ore veins** — sparse 3D noise blobs replace stone with ore types within elevation bands.

All noise seeded from a `u64` world seed in `WorldConfig`. Chunk positions are folded modulo world extent before sampling, so heightmap tiles seamlessly at the wrapping boundary.

**`FlatGenerator`**
Generates a flat platform of a configurable material to a configurable depth. Used for sandbox mode and tests.

### Game Mode Independence

`WorldGenerator` and `GameMode` are independent resources. Any generator can pair with any game mode:
- `ProceduralGenerator` + `Survival` — normal gameplay
- `ProceduralGenerator` + `Creative` — terrain exploration without resource constraints
- `FlatGenerator` + `Creative` — canonical sandbox/debug environment
- `FlatGenerator` + `Survival` — useful for isolated crafting/mechanic tests

---

## Section 6: Player & Controls

Built on `avian3d`, extending the existing `avian_3d_character` crate in this monorepo. First-person capsule with walk, jump, crouch, and gravity. Collision runs against static chunk `Collider`s only.

**Voxel targeting** uses a raycast against the static chunk voxel grid (not physics colliders) for sub-voxel precision at low cost. The targeted voxel gets a wireframe outline overlay.

| Input | Action |
|---|---|
| Left click | Swing active tool — breaks targeted voxel, ejects debris |
| Right click | Place selected inventory item at targeted face |
| Scroll / hotbar keys | Cycle active inventory slot |
| E | Open inventory / crafting UI |
| F | Pick up nearby debris within reach distance |

---

## Section 7: Persistence

### Chunk Files

One binary file per modified chunk, named by position (e.g. `chunks/0_-3_2.bin`). Content is the raw `[VoxelId; CHUNK_SIZE³]` array serialized with `bincode`. Unmodified chunks are never written to disk — they are regenerated from the world seed on load.

### WorldManifest

A RON file storing:
- World seed
- `CHUNK_SIZE` and `VOXEL_SIZE` (mismatch on load = hard error)
- World extent (wrapping bounds)
- List of dirty (modified) chunk positions
- `GameMode`

Player state (position, inventory) is a separate small RON file.

---

## Section 8: Testing

**Unit tests** (`cargo test`) — pure functions with no Bevy app:
- Greedy mesher correctness
- Noise sampling and heightmap generation
- Recipe lookup
- Coordinate conversion (`ChunkPos` ↔ world-space)
- CA simulation step

**Integration tests** — minimal Bevy `App` with `FlatGenerator`:
- Chunk load/unload lifecycle
- Break voxel → debris eject → CA fall → solidify back into chunk
- Debris pick up → inventory increment
- Crafting recipe fulfillment

**Benchmarks** (`cargo bench`, criterion):
- Greedy mesher at `CHUNK_SIZE` = 16, 32, 64, 128
- CA tick throughput vs. active debris count
- Chunk generation speed (`ProceduralGenerator` vs. `FlatGenerator`)

Sandbox/creative mode (`FlatGenerator` + `Creative`) serves as the manual golden-path test environment.

---

## Out of Scope (Future Sessions)

- Enemies / hostile AI
- Player needs (hunger, thirst)
- Multiplayer
- Biomes beyond material layering
- Structural integrity / chunk collapse
- Brickmap GI/shadow layer
- Fluid simulation beyond CA debris
