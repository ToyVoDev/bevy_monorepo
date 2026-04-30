# Spawn World Loading Design

## Goal

Prevent the player from falling into the void at game start by generating a configurable radius of chunks around the spawn point before the player enters the gameplay state.

## State Flow

```
Splash → Title → Loading (assets) → WorldLoading (chunks) → Gameplay
```

The existing `Loading` screen's transition target changes from `Screen::Gameplay` to `Screen::WorldLoading`. Everything else in `Loading` is unchanged.

## Constants

- `CHUNK_SIZE = 32`, `VOXEL_SIZE = 0.1m` → 1 chunk = 3.2m per side
- Spawn XZ world position: `(0.5, _, 0.5)` → chunk `(0, _, 0)`, local voxel `(5, _, 5)`
- Default `spawn_radius = 3` → 7×7×7 cube of chunks = 49 columns × 7 Y levels

## Settings

`spawn_radius: u32` added to `Settings` in `src/lib.rs`. Default: `3`. Represents the Chebyshev radius in chunk units around `ChunkPos(0,0,0)`.

## WorldLoading Screen

**File:** `src/ui/screens/world_loading.rs` (new)

**`OnEnter(Screen::WorldLoading)`:**
- Spawn a "Generating world…" label entity tagged `DespawnOnExit(Screen::WorldLoading)`
- Populate `PendingGeneration` with all `ChunkPos` within `spawn_radius` of `ChunkPos(0,0,0)` in all three axes. `load_unload_chunks` is inactive (no player), so this queue persists until consumed.
- Sort the queue surface-first (same Y-penalty heuristic as the existing `load_unload_chunks` sort) so the chunks the player will land on generate first.

**`Update` while in `Screen::WorldLoading`:**
- `spawn_area_ready` run condition: returns `true` when every `ChunkPos` within `spawn_radius` of `(0,0,0)` is present in `ChunkedWorld`.
- When ready: transition to `Screen::Gameplay`.

## Chunk Pipeline in WorldLoading

`PausableSystems` is gated on `in_state(Screen::Gameplay)` in `main.rs`, so the chunk systems are idle during `WorldLoading`. Fix: add a second `add_systems` call in `ChunkPlugin::build` that registers the same generation and meshing pipeline under `in_state(Screen::WorldLoading)`:

```
spawn_generation_tasks (after load_unload_chunks is a no-op, so no after() needed)
collect_generated_chunks (after spawn_generation_tasks)
spawn_meshing_tasks (after collect_generated_chunks)
collect_meshed_chunks (after spawn_meshing_tasks)
```

No pause check is needed — you cannot pause a loading screen. The existing `PausableSystems` registration in `ChunkPlugin` is unchanged. LOD systems do not run in `WorldLoading`.

## Surface Finding

`spawn_player` in `src/player/controller.rs` gains `Res<ChunkedWorld>` and `Res<Settings>` parameters. It calls `find_spawn_y(&world)` to compute the player's initial Y:

- Scan chunks at `ChunkPos(0, y, 0)` from `y = spawn_radius as i32` down to `-(spawn_radius as i32)`
- Within each chunk, scan local Y from top (`CHUNK_SIZE - 1`) to bottom (`0`) at local XZ `(5, 5)`
- On the first solid voxel: compute world Y and return it plus `1.0` (1 metre clearance, enough for the 0.9m capsule collider)
- Fallback: `5.0` (current hardcoded value) if no solid voxel is found

## Files

| File | Change |
|------|--------|
| `src/lib.rs` | Add `spawn_radius: u32` to `Settings` (default `3`) |
| `src/ui/screens/mod.rs` | Add `WorldLoading` variant to `Screen`; register `world_loading::plugin` |
| `src/ui/screens/loading.rs` | Transition to `Screen::WorldLoading` instead of `Screen::Gameplay` |
| `src/ui/screens/world_loading.rs` | New: spawn UI, seed `PendingGeneration`, `spawn_area_ready` condition, transition |
| `src/chunk/mod.rs` | Second `add_systems` for generation + meshing pipeline in `WorldLoading` |
| `src/player/controller.rs` | `spawn_player` computes Y via surface scan instead of hardcoded `5.0` |

## Out of Scope

- Progress bar on the loading screen (text label only)
- Configuring spawn XZ position (always `(0.5, _, 0.5)`)
- Waiting for LOD1/LOD2 super-chunks during world loading
