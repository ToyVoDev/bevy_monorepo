# Spawn World Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `WorldLoading` screen between asset loading and gameplay that pre-generates a configurable radius of chunks around the spawn point so the player never falls into the void.

**Architecture:** A new `Screen::WorldLoading` state seeds `PendingGeneration` with spawn-radius chunks and waits for all of them to appear in `ChunkedWorld` before transitioning to gameplay. The chunk generation + meshing pipeline runs in this new state via a separate `add_systems` call that bypasses `PausableSystems`. `spawn_player` is promoted to a standalone Bevy system that scans `ChunkedWorld` to find the actual surface Y.

**Tech Stack:** Bevy 0.18, Rust 2024, `avian3d`, existing `ChunkedWorld` / `PendingGeneration` resources.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/lib.rs` | Modify | Add `spawn_radius: u32` to `Settings` |
| `src/ui/screens/mod.rs` | Modify | Add `WorldLoading` variant; register new plugin |
| `src/ui/screens/loading.rs` | Modify | Redirect transition target to `WorldLoading` |
| `src/ui/screens/world_loading.rs` | Create | Spawn UI, seed queue, readiness check, transition |
| `src/chunk/mod.rs` | Modify | Run generation + meshing pipeline in `WorldLoading` |
| `src/player/controller.rs` | Modify | `find_spawn_y` pure fn; `spawn_player` reads surface Y |
| `src/ui/screens/gameplay.rs` | Modify | Register `spawn_player` as standalone system; remove inline call |

---

## Task 1: Add `spawn_radius` to Settings and `WorldLoading` to Screen

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/ui/screens/mod.rs`
- Modify: `src/ui/screens/loading.rs`

---

- [ ] **Step 1: Write failing test for Settings default**

Append to the bottom of `src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_spawn_radius() {
        assert_eq!(Settings::default().spawn_radius, 3);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd voxel_game && cargo test --lib settings_default_spawn_radius 2>&1 | tail -5
```

Expected: compile error — `spawn_radius` field doesn't exist yet.

- [ ] **Step 3: Add `spawn_radius` to Settings**

In `src/lib.rs`, update the `Settings` struct and its `Default` impl:

```rust
#[derive(Resource, Clone)]
pub struct Settings {
    pub master_volume: f32,
    pub mouse_sensitivity: f32,
    pub fov: f32,
    pub render_distance: u32,
    pub show_coordinates: bool,
    pub spawn_radius: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            mouse_sensitivity: 1.0,
            fov: 90.0,
            render_distance: 8,
            show_coordinates: false,
            spawn_radius: 3,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd voxel_game && cargo test --lib settings_default_spawn_radius 2>&1 | tail -5
```

Expected: PASS.

- [ ] **Step 5: Add `WorldLoading` variant to Screen**

In `src/ui/screens/mod.rs`, update the `Screen` enum and plugin registration:

```rust
mod gameplay;
mod loading;
mod splash;
mod title;
mod world_loading;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Screen>();

    app.add_plugins((
        gameplay::plugin,
        loading::plugin,
        splash::plugin,
        title::plugin,
        world_loading::plugin,
    ));
}

/// The game's main screen states.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Screen {
    #[default]
    Splash,
    Title,
    Loading,
    WorldLoading,
    Gameplay,
}
```

- [ ] **Step 6: Redirect Loading → WorldLoading**

In `src/ui/screens/loading.rs`, change `enter_gameplay_screen`:

```rust
fn enter_gameplay_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::WorldLoading);
}
```

(Only the argument to `set` changes; nothing else in the file changes.)

- [ ] **Step 7: Build to verify it compiles (world_loading module is empty — create a stub)**

Create `src/ui/screens/world_loading.rs` with a minimal stub so the `mod world_loading;` declaration compiles:

```rust
use bevy::prelude::*;

pub(super) fn plugin(_app: &mut App) {}
```

```bash
cd voxel_game && cargo build 2>&1 | grep "^error" | head -10
```

Expected: zero errors.

- [ ] **Step 8: Commit**

```bash
cd /Users/CollinDie/Code/bevy_monorepo
git add voxel_game/src/lib.rs voxel_game/src/ui/screens/mod.rs \
        voxel_game/src/ui/screens/loading.rs voxel_game/src/ui/screens/world_loading.rs
git commit -m "feat: add spawn_radius setting and WorldLoading screen variant"
```

---

## Task 2: Create world_loading.rs screen

**Files:**
- Modify: `src/ui/screens/world_loading.rs` (replace stub)

The readiness check logic is extracted into a pure function `all_spawn_chunks_present` so it can be unit-tested without a full Bevy app.

---

- [ ] **Step 1: Write failing tests**

Replace the stub `src/ui/screens/world_loading.rs` entirely with:

```rust
use bevy::prelude::*;
use crate::ui::screens::Screen;
use crate::chunk::loading::{ChunkedWorld, PendingGeneration};
use crate::types::ChunkPos;
use crate::Settings;

pub(super) fn plugin(app: &mut App) {}

/// Returns true when every ChunkPos within `radius` of origin is in `world`.
pub fn all_spawn_chunks_present(world: &ChunkedWorld, radius: u32) -> bool {
    let r = radius as i32;
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                if !world.chunks.contains_key(&ChunkPos(dx, dy, dz)) {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;

    #[test]
    fn empty_world_not_ready() {
        let world = ChunkedWorld::default();
        assert!(!all_spawn_chunks_present(&world, 1));
    }

    #[test]
    fn partial_world_not_ready() {
        let mut world = ChunkedWorld::default();
        // radius=1 requires 3³=27 chunks; inserting only 1 is not enough
        world.chunks.insert(ChunkPos(0, 0, 0), Chunk::new());
        assert!(!all_spawn_chunks_present(&world, 1));
    }

    #[test]
    fn full_radius_1_is_ready() {
        let mut world = ChunkedWorld::default();
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                for dz in -1i32..=1 {
                    world.chunks.insert(ChunkPos(dx, dy, dz), Chunk::new());
                }
            }
        }
        assert!(all_spawn_chunks_present(&world, 1));
    }

    #[test]
    fn radius_0_only_needs_origin() {
        let mut world = ChunkedWorld::default();
        world.chunks.insert(ChunkPos(0, 0, 0), Chunk::new());
        assert!(all_spawn_chunks_present(&world, 0));
    }
}
```

- [ ] **Step 2: Run tests to verify they compile but the logic tests may fail**

```bash
cd voxel_game && cargo test --lib world_loading 2>&1 | tail -10
```

Expected: all 4 tests pass immediately (the logic is already in the file — TDD confirms the function behaves correctly before wiring it up).

- [ ] **Step 3: Implement the full plugin**

Replace the stub `pub(super) fn plugin(_app: &mut App) {}` with the full implementation:

```rust
pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::WorldLoading), enter_world_loading);
    app.add_systems(
        Update,
        check_spawn_ready.run_if(in_state(Screen::WorldLoading)),
    );
}

fn enter_world_loading(
    mut commands: Commands,
    settings: Res<Settings>,
    mut pending: ResMut<PendingGeneration>,
) {
    commands.spawn((
        widget::ui_root("World Loading Screen"),
        DespawnOnExit(Screen::WorldLoading),
        children![widget::label("Generating world...")],
    ));

    let r = settings.spawn_radius as i32;
    pending.0.clear();
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                pending.0.push_back(ChunkPos(dx, dy, dz));
            }
        }
    }
    // Surface-first: prioritise chunks near Y=0 and above
    pending.0.make_contiguous().sort_unstable_by_key(|p| {
        let xz = p.0.abs() + p.2.abs();
        let dy = p.1;
        let y_cost = if dy < 0 { (-dy) * 4 } else { dy };
        xz + y_cost
    });
}

fn check_spawn_ready(
    world: Res<ChunkedWorld>,
    settings: Res<Settings>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if all_spawn_chunks_present(&world, settings.spawn_radius) {
        next_screen.set(Screen::Gameplay);
    }
}
```

Also add the `widget` import at the top of the file (after the existing `use` statements):

```rust
use crate::ui::theme::widget;
```

The complete `world_loading.rs` after this step:

```rust
use bevy::prelude::*;
use crate::ui::screens::Screen;
use crate::ui::theme::widget;
use crate::chunk::loading::{ChunkedWorld, PendingGeneration};
use crate::types::ChunkPos;
use crate::Settings;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::WorldLoading), enter_world_loading);
    app.add_systems(
        Update,
        check_spawn_ready.run_if(in_state(Screen::WorldLoading)),
    );
}

fn enter_world_loading(
    mut commands: Commands,
    settings: Res<Settings>,
    mut pending: ResMut<PendingGeneration>,
) {
    commands.spawn((
        widget::ui_root("World Loading Screen"),
        DespawnOnExit(Screen::WorldLoading),
        children![widget::label("Generating world...")],
    ));

    let r = settings.spawn_radius as i32;
    pending.0.clear();
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                pending.0.push_back(ChunkPos(dx, dy, dz));
            }
        }
    }
    pending.0.make_contiguous().sort_unstable_by_key(|p| {
        let xz = p.0.abs() + p.2.abs();
        let dy = p.1;
        let y_cost = if dy < 0 { (-dy) * 4 } else { dy };
        xz + y_cost
    });
}

fn check_spawn_ready(
    world: Res<ChunkedWorld>,
    settings: Res<Settings>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if all_spawn_chunks_present(&world, settings.spawn_radius) {
        next_screen.set(Screen::Gameplay);
    }
}

pub fn all_spawn_chunks_present(world: &ChunkedWorld, radius: u32) -> bool {
    let r = radius as i32;
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                if !world.chunks.contains_key(&ChunkPos(dx, dy, dz)) {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;

    #[test]
    fn empty_world_not_ready() {
        let world = ChunkedWorld::default();
        assert!(!all_spawn_chunks_present(&world, 1));
    }

    #[test]
    fn partial_world_not_ready() {
        let mut world = ChunkedWorld::default();
        world.chunks.insert(ChunkPos(0, 0, 0), Chunk::new());
        assert!(!all_spawn_chunks_present(&world, 1));
    }

    #[test]
    fn full_radius_1_is_ready() {
        let mut world = ChunkedWorld::default();
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                for dz in -1i32..=1 {
                    world.chunks.insert(ChunkPos(dx, dy, dz), Chunk::new());
                }
            }
        }
        assert!(all_spawn_chunks_present(&world, 1));
    }

    #[test]
    fn radius_0_only_needs_origin() {
        let mut world = ChunkedWorld::default();
        world.chunks.insert(ChunkPos(0, 0, 0), Chunk::new());
        assert!(all_spawn_chunks_present(&world, 0));
    }
}
```

- [ ] **Step 4: Build to verify it compiles**

```bash
cd voxel_game && cargo build 2>&1 | grep "^error" | head -10
```

Expected: zero errors. If `widget::ui_root` or `widget::label` don't exist, check `src/ui/theme/widget.rs` for the actual function names and use those.

- [ ] **Step 5: Run all tests**

```bash
cd voxel_game && cargo test --lib 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/CollinDie/Code/bevy_monorepo
git add voxel_game/src/ui/screens/world_loading.rs
git commit -m "feat: add WorldLoading screen with spawn area readiness check"
```

---

## Task 3: Wire chunk pipeline to WorldLoading state

**Files:**
- Modify: `src/chunk/mod.rs`

The chunk generation and meshing systems currently only run inside `PausableSystems`, which is gated on `Screen::Gameplay`. We need them to also run during `Screen::WorldLoading`.

---

- [ ] **Step 1: Add the WorldLoading systems**

In `src/chunk/mod.rs`, add a `use` for `Screen` and a second `add_systems` call inside `ChunkPlugin::build`. The final `build` method:

```rust
fn build(&self, app: &mut App) {
    use crate::ui::screens::Screen;

    app
        .init_resource::<ChunkedWorld>()
        .init_resource::<ChunkEntities>()
        .init_resource::<PendingGeneration>()
        .init_resource::<GeneratingChunks>()
        .init_resource::<MeshingChunks>()
        .init_resource::<SuperChunkedWorld>()
        .init_resource::<SuperChunkEntities>()
        .init_resource::<PendingSuperChunks>()
        .init_resource::<MeshingLodChunks>()
        .add_systems(Update, (
            load_unload_chunks,
            spawn_generation_tasks.after(load_unload_chunks),
            collect_generated_chunks.after(spawn_generation_tasks),
            spawn_meshing_tasks.after(collect_generated_chunks).after(spawn_lod_meshing_tasks),
            collect_meshed_chunks.after(spawn_meshing_tasks),
            lod_coordinator.after(load_unload_chunks),
            spawn_lod_meshing_tasks.after(lod_coordinator).after(collect_generated_chunks),
            collect_lod_meshed_chunks.after(spawn_lod_meshing_tasks),
        ).in_set(PausableSystems))
        .add_systems(Update, (
            spawn_generation_tasks,
            collect_generated_chunks.after(spawn_generation_tasks),
            spawn_meshing_tasks.after(collect_generated_chunks),
            collect_meshed_chunks.after(spawn_meshing_tasks),
        ).run_if(in_state(Screen::WorldLoading)));
}
```

- [ ] **Step 2: Build to verify it compiles**

```bash
cd voxel_game && cargo build 2>&1 | grep "^error" | head -10
```

Expected: zero errors.

- [ ] **Step 3: Run all tests**

```bash
cd voxel_game && cargo test --lib 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/CollinDie/Code/bevy_monorepo
git add voxel_game/src/chunk/mod.rs
git commit -m "feat: run chunk generation pipeline during WorldLoading state"
```

---

## Task 4: Surface finding and spawn_player update

**Files:**
- Modify: `src/player/controller.rs`
- Modify: `src/ui/screens/gameplay.rs`

`spawn_player` becomes a standalone Bevy system (removed from `spawn_gameplay`). A pure `find_spawn_y` function scans `ChunkedWorld` for the surface at spawn XZ.

---

- [ ] **Step 1: Write failing tests**

Append to `src/player/controller.rs` (before the closing brace of the file, or at the end):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::loading::ChunkedWorld;
    use crate::chunk::Chunk;
    use crate::types::{ChunkPos, LocalVoxelPos, STONE};

    #[test]
    fn find_spawn_y_finds_surface() {
        let mut world = ChunkedWorld::default();
        let mut chunk = Chunk::new();
        // Solid voxel at local (5, 10, 5) in chunk (0, 0, 0)
        chunk.set(LocalVoxelPos::new(5, 10, 5), STONE);
        world.chunks.insert(ChunkPos(0, 0, 0), chunk);
        let y = find_spawn_y(&world, 3);
        // World Y of top face = (0 * 32 + 10 + 1) * 0.1 = 1.1; +1.0 clearance = 2.1
        assert!((y - 2.1).abs() < 1e-4, "expected 2.1, got {y}");
    }

    #[test]
    fn find_spawn_y_fallback_when_no_solid() {
        let world = ChunkedWorld::default();
        assert_eq!(find_spawn_y(&world, 3), 5.0);
    }

    #[test]
    fn find_spawn_y_prefers_higher_chunk() {
        let mut world = ChunkedWorld::default();
        // Solid voxel at local (5, 0, 5) in chunk (0, 1, 0) — higher than chunk (0, 0, 0)
        let mut upper = Chunk::new();
        upper.set(LocalVoxelPos::new(5, 0, 5), STONE);
        world.chunks.insert(ChunkPos(0, 1, 0), upper);
        let mut lower = Chunk::new();
        lower.set(LocalVoxelPos::new(5, 31, 5), STONE);
        world.chunks.insert(ChunkPos(0, 0, 0), lower);
        let y = find_spawn_y(&world, 3);
        // Upper chunk (chunk_y=1, local_y=0): world Y = (1*32 + 0 + 1)*0.1 + 1.0 = 3.3 + 1.0 = 4.3
        // Lower chunk (chunk_y=0, local_y=31): world Y = (0*32 + 31 + 1)*0.1 + 1.0 = 3.2 + 1.0 = 4.2
        // scan goes from y=3 down, hits chunk (0,1,0) first
        assert!((y - 4.3).abs() < 1e-4, "expected 4.3, got {y}");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd voxel_game && cargo test --lib find_spawn_y 2>&1 | tail -5
```

Expected: compile error — `find_spawn_y` not defined.

- [ ] **Step 3: Implement `find_spawn_y` and update `spawn_player`**

Replace the entire contents of `src/player/controller.rs` with:

```rust
use bevy::prelude::*;
use avian3d::prelude::*;
use super::Player;
use crate::chunk::loading::ChunkedWorld;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{ChunkPos, LocalVoxelPos};
use crate::Settings;
use crate::ui::screens::Screen;

/// Scans the Y column at spawn XZ (chunk 0,_,0 local voxel 5,_,5) top-down
/// and returns 1m above the first solid voxel, or 5.0 if none is found.
pub fn find_spawn_y(world: &ChunkedWorld, spawn_radius: u32) -> f32 {
    let r = spawn_radius as i32;
    for chunk_y in (-r..=r).rev() {
        let chunk_pos = ChunkPos(0, chunk_y, 0);
        if let Some(chunk) = world.get(chunk_pos) {
            for local_y in (0..CHUNK_SIZE).rev() {
                let vp = LocalVoxelPos::new(5, local_y as u8, 5);
                if chunk.is_solid(vp) {
                    let world_y = (chunk_y as f32 * CHUNK_SIZE as f32
                        + local_y as f32
                        + 1.0)
                        * VOXEL_SIZE
                        + 1.0;
                    return world_y;
                }
            }
        }
    }
    5.0
}

pub fn spawn_player(
    mut commands: Commands,
    world: Res<ChunkedWorld>,
    settings: Res<Settings>,
) {
    let y = find_spawn_y(&world, settings.spawn_radius);
    commands.spawn((
        Player,
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.9),
        LockedAxes::ROTATION_LOCKED,
        LinearDamping(0.0),
        Friction::ZERO,
        Restitution::ZERO,
        GravityScale(1.0),
        LinearVelocity::default(),
        Transform::from_xyz(0.5, y, 0.5),
        Visibility::Hidden,
        DespawnOnExit(Screen::Gameplay),
    ));
}

pub fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, With<super::camera::PlayerCamera>>,
    mut player_query: Query<(&Transform, &mut LinearVelocity), With<Player>>,
) {
    let Ok(cam_transform) = camera_query.single() else { return };
    let Ok((_, mut velocity)) = player_query.single_mut() else { return };

    let speed = 5.0_f32;
    let jump_impulse = 7.0_f32;

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

    if keys.just_pressed(KeyCode::Space) && velocity.y.abs() < 0.1 {
        velocity.y = jump_impulse;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::loading::ChunkedWorld;
    use crate::chunk::Chunk;
    use crate::types::{ChunkPos, LocalVoxelPos, STONE};

    #[test]
    fn find_spawn_y_finds_surface() {
        let mut world = ChunkedWorld::default();
        let mut chunk = Chunk::new();
        chunk.set(LocalVoxelPos::new(5, 10, 5), STONE);
        world.chunks.insert(ChunkPos(0, 0, 0), chunk);
        let y = find_spawn_y(&world, 3);
        // World Y of top face = (0*32 + 10 + 1)*0.1 + 1.0 = 1.1 + 1.0 = 2.1
        assert!((y - 2.1).abs() < 1e-4, "expected 2.1, got {y}");
    }

    #[test]
    fn find_spawn_y_fallback_when_no_solid() {
        let world = ChunkedWorld::default();
        assert_eq!(find_spawn_y(&world, 3), 5.0);
    }

    #[test]
    fn find_spawn_y_prefers_higher_chunk() {
        let mut world = ChunkedWorld::default();
        let mut upper = Chunk::new();
        upper.set(LocalVoxelPos::new(5, 0, 5), STONE);
        world.chunks.insert(ChunkPos(0, 1, 0), upper);
        let mut lower = Chunk::new();
        lower.set(LocalVoxelPos::new(5, 31, 5), STONE);
        world.chunks.insert(ChunkPos(0, 0, 0), lower);
        let y = find_spawn_y(&world, 3);
        // chunk (0,1,0), local_y=0: (1*32 + 0 + 1)*0.1 + 1.0 = 3.3 + 1.0 = 4.3
        assert!((y - 4.3).abs() < 1e-4, "expected 4.3, got {y}");
    }
}
```

- [ ] **Step 4: Remove `spawn_player` call from `spawn_gameplay` and register it as a standalone system**

In `src/ui/screens/gameplay.rs`:

**Remove** the line `player::controller::spawn_player(commands.reborrow());` from `spawn_gameplay`.

**Add** `player::controller::spawn_player` to the `OnEnter(Screen::Gameplay)` system tuple:

```rust
pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), (
        set_gameplay_bg,
        spawn_gameplay,
        crate::despawn_ui_camera,
        player::camera::spawn_camera,
        player::hud::spawn_highlight,
        player::controller::spawn_player,
    ));
    // ... rest unchanged
```

The `spawn_gameplay` function signature loses its `player::controller::spawn_player` call but otherwise stays the same. The two changes to `spawn_gameplay`:

1. Remove: `player::controller::spawn_player(commands.reborrow());`
2. That's it — nothing else changes.

- [ ] **Step 5: Run tests**

```bash
cd voxel_game && cargo test --lib 2>&1 | tail -10
```

Expected: all tests pass (48 + 3 new = 51 total, or similar count).

- [ ] **Step 6: Verify build is clean**

```bash
cd voxel_game && cargo build 2>&1 | grep "^error" | head -10
```

Expected: zero errors.

- [ ] **Step 7: Commit**

```bash
cd /Users/CollinDie/Code/bevy_monorepo
git add voxel_game/src/player/controller.rs voxel_game/src/ui/screens/gameplay.rs
git commit -m "feat: surface-finding spawn_player, no more hardcoded Y=5"
```

---

## Self-Review Checklist

**Spec coverage:**
- [x] `spawn_radius: u32` added to `Settings` (default 3) → Task 1
- [x] `Screen::WorldLoading` added → Task 1
- [x] `Loading` redirects to `WorldLoading` → Task 1
- [x] `world_loading.rs`: spawn UI, seed `PendingGeneration`, sort surface-first → Task 2
- [x] `all_spawn_chunks_present` condition → Task 2
- [x] Transition to `Screen::Gameplay` when ready → Task 2
- [x] Chunk pipeline runs in `WorldLoading` → Task 3
- [x] LOD systems do NOT run in `WorldLoading` → Task 3 (not added)
- [x] `find_spawn_y` scans column at (0, _, 0) local (5, _, 5) → Task 4
- [x] Fallback to 5.0 → Task 4
- [x] `spawn_player` uses surface Y → Task 4

**Type consistency:**
- `find_spawn_y(world: &ChunkedWorld, spawn_radius: u32) -> f32` — used consistently in test and implementation
- `all_spawn_chunks_present(world: &ChunkedWorld, radius: u32) -> bool` — used consistently
- `ChunkPos(dx, dy, dz)` tuple struct — matches `types.rs`
- `LocalVoxelPos::new(5, local_y as u8, 5)` — `as u8` needed because `local_y` is `usize`
