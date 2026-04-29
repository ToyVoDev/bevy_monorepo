# Voxel Game Phase 2 — Gameplay Mechanics

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add voxel break/place, falling debris CA simulation, inventory pickup, tools, crafting, and ProceduralGenerator on top of the Phase 1 foundation.

**Architecture:** Voxel targeting via grid raycast. Breaking a voxel ejects a `DebrisParticle` entity into a cellular automata simulation (ported from `ProjectSandBevy`) that falls, settles, and either gets picked up or solidifies back into the static chunk. Inventory is a fixed slot list. Recipes are loaded from `assets/recipes.ron`.

**Tech Stack:** Bevy 0.16, avian3d 0.3, noise 0.9 (new), ron 0.8 (new), bincode 1.3 (new)

**Prerequisite:** Phase 1 complete and all tests passing.

---

## File Map

| File | Responsibility |
|---|---|
| `src/player/interaction.rs` | Grid raycast, voxel targeting, break/place input, F-pickup |
| `src/simulation/mod.rs` | `SimulationPlugin` |
| `src/simulation/debris.rs` | `DebrisParticle` component, CA tick, solidify timer |
| `src/inventory/mod.rs` | `Inventory` resource, `InventorySlot` |
| `src/inventory/crafting.rs` | `Recipe`, RON loading, crafting system |
| `src/inventory/ui.rs` | Hotbar HUD, E-key inventory panel |
| `src/world/procedural.rs` | `ProceduralGenerator` |
| `benches/generation.rs` | Criterion benchmark: FlatGenerator vs ProceduralGenerator |
| `assets/recipes.ron` | Recipe data file |

---

## Task 1: Add Dependencies

**Files:**
- Modify: `voxel_game/Cargo.toml`

- [ ] **Step 1: Add noise, ron, bincode**

In `voxel_game/Cargo.toml`, extend `[dependencies]`:

```toml
[dependencies]
bevy = "0.16"
avian3d = { version = "0.3", features = ["3d", "f32", "parry-f32"] }
serde = { version = "1", features = ["derive"] }
noise = "0.9"
ron = "0.8"
bincode = "1.3"
```

- [ ] **Step 2: Verify compile**

```bash
cargo build -p voxel_game
```

Expected: compiles (new deps download and build)

- [ ] **Step 3: Commit**

```bash
git add voxel_game/Cargo.toml
git commit -m "feat(voxel_game): add noise/ron/bincode dependencies"
```

---

## Task 2: Voxel Targeting (Grid Raycast)

**Files:**
- Create: `voxel_game/src/player/interaction.rs`
- Modify: `voxel_game/src/player/mod.rs`

- [ ] **Step 1: Write failing test for raycast**

Create `voxel_game/src/player/interaction.rs`:

```rust
use bevy::prelude::*;
use crate::chunk::loading::ChunkedWorld;
use crate::config::VOXEL_SIZE;
use crate::types::{ChunkPos, LocalVoxelPos, VoxelId, AIR};

pub const REACH: f32 = 5.0;

/// Result of a voxel grid raycast.
#[derive(Debug, PartialEq)]
pub struct VoxelHit {
    pub chunk: ChunkPos,
    pub local: LocalVoxelPos,
    pub voxel_id: VoxelId,
    /// The face normal (unit vector, one axis only)
    pub normal: IVec3,
}

/// Cast a ray from `origin` in `direction`, step through the voxel grid,
/// and return the first solid voxel hit within `max_dist`.
pub fn raycast_voxels(
    world: &ChunkedWorld,
    origin: Vec3,
    direction: Vec3,
    max_dist: f32,
) -> Option<VoxelHit> {
    // DDA voxel traversal (Amanatides & Woo)
    let dir = direction.normalize();
    let mut pos = (origin / VOXEL_SIZE).floor().as_ivec3();
    let step = IVec3::new(
        if dir.x >= 0.0 { 1 } else { -1 },
        if dir.y >= 0.0 { 1 } else { -1 },
        if dir.z >= 0.0 { 1 } else { -1 },
    );
    let delta = Vec3::new(
        (1.0 / dir.x.abs()).min(1e30),
        (1.0 / dir.y.abs()).min(1e30),
        (1.0 / dir.z.abs()).min(1e30),
    ) * VOXEL_SIZE;
    let origin_voxel = (origin / VOXEL_SIZE).floor();
    let mut t_max = Vec3::new(
        if dir.x >= 0.0 { (origin_voxel.x + 1.0) - origin.x / VOXEL_SIZE }
                   else { origin.x / VOXEL_SIZE - origin_voxel.x } ,
        if dir.y >= 0.0 { (origin_voxel.y + 1.0) - origin.y / VOXEL_SIZE }
                   else { origin.y / VOXEL_SIZE - origin_voxel.y },
        if dir.z >= 0.0 { (origin_voxel.z + 1.0) - origin.z / VOXEL_SIZE }
                   else { origin.z / VOXEL_SIZE - origin_voxel.z },
    ) * VOXEL_SIZE;
    let mut last_normal = IVec3::ZERO;
    let mut dist = 0.0_f32;

    while dist < max_dist {
        // Look up voxel
        let n = crate::config::CHUNK_SIZE as i32;
        let chunk_pos = ChunkPos(
            pos.x.div_euclid(n),
            pos.y.div_euclid(n),
            pos.z.div_euclid(n),
        );
        let lx = pos.x.rem_euclid(n) as u8;
        let ly = pos.y.rem_euclid(n) as u8;
        let lz = pos.z.rem_euclid(n) as u8;
        let local = LocalVoxelPos::new(lx, ly, lz);

        if let Some(chunk) = world.get(chunk_pos) {
            let voxel_id = chunk.get(local);
            if voxel_id != AIR {
                return Some(VoxelHit { chunk: chunk_pos, local, voxel_id, normal: last_normal });
            }
        }

        // Advance to next voxel boundary
        if t_max.x < t_max.y && t_max.x < t_max.z {
            dist = t_max.x;
            t_max.x += delta.x;
            pos.x += step.x;
            last_normal = IVec3::new(-step.x, 0, 0);
        } else if t_max.y < t_max.z {
            dist = t_max.y;
            t_max.y += delta.y;
            pos.y += step.y;
            last_normal = IVec3::new(0, -step.y, 0);
        } else {
            dist = t_max.z;
            t_max.z += delta.z;
            pos.z += step.z;
            last_normal = IVec3::new(0, 0, -step.z);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::loading::ChunkedWorld;
    use crate::chunk::Chunk;
    use crate::types::STONE;

    fn world_with_stone_at_voxel(vx: i32, vy: i32, vz: i32) -> ChunkedWorld {
        let n = crate::config::CHUNK_SIZE as i32;
        let chunk_pos = ChunkPos(vx.div_euclid(n), vy.div_euclid(n), vz.div_euclid(n));
        let local = LocalVoxelPos::new(
            vx.rem_euclid(n) as u8,
            vy.rem_euclid(n) as u8,
            vz.rem_euclid(n) as u8,
        );
        let mut chunk = Chunk::new();
        chunk.set(local, STONE);
        let mut w = ChunkedWorld::default();
        w.chunks.insert(chunk_pos, chunk);
        w
    }

    #[test]
    fn ray_hits_stone_directly_ahead() {
        // Stone at voxel (5, 0, 0). Ray from origin pointing +X.
        let world = world_with_stone_at_voxel(5, 0, 0);
        let origin = Vec3::new(0.05, 0.05, 0.05); // inside voxel (0,0,0)
        let hit = raycast_voxels(&world, origin, Vec3::X, REACH);
        assert!(hit.is_some(), "should hit the stone");
        let hit = hit.unwrap();
        assert_eq!(hit.voxel_id, STONE);
    }

    #[test]
    fn ray_misses_when_nothing_in_path() {
        let world = ChunkedWorld::default();
        let hit = raycast_voxels(&world, Vec3::ZERO, Vec3::X, REACH);
        assert!(hit.is_none());
    }

    #[test]
    fn ray_beyond_reach_does_not_hit() {
        // Stone far away, beyond REACH
        let world = world_with_stone_at_voxel(100, 0, 0);
        let origin = Vec3::new(0.05, 0.05, 0.05);
        let hit = raycast_voxels(&world, origin, Vec3::X, REACH);
        assert!(hit.is_none());
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test -p voxel_game interaction
```

Expected: FAIL — compile errors (module not registered yet) or logic failures

- [ ] **Step 3: Register interaction module in player**

In `voxel_game/src/player/mod.rs`, add:
```rust
pub mod interaction;
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p voxel_game interaction
```

Expected: all 3 tests PASS

- [ ] **Step 5: Add targeting system (highlight)**

Append to `voxel_game/src/player/interaction.rs`:

```rust
#[derive(Resource, Default)]
pub struct TargetedVoxel(pub Option<VoxelHit>);

pub fn update_targeted_voxel(
    camera_query: Query<&Transform, With<crate::player::camera::PlayerCamera>>,
    world: Res<ChunkedWorld>,
    mut targeted: ResMut<TargetedVoxel>,
) {
    let Ok(cam) = camera_query.get_single() else {
        targeted.0 = None;
        return;
    };
    targeted.0 = raycast_voxels(&world, cam.translation, cam.forward().into(), REACH);
}
```

Register in `PlayerPlugin::build` in `player/mod.rs`:
```rust
app
    .init_resource::<interaction::TargetedVoxel>()
    .add_systems(Update, interaction::update_targeted_voxel);
```

- [ ] **Step 6: Commit**

```bash
git add voxel_game/src/player/interaction.rs voxel_game/src/player/mod.rs
git commit -m "feat(voxel_game): voxel grid raycast and targeted voxel"
```

---

## Task 3: Inventory

**Files:**
- Create: `voxel_game/src/inventory/mod.rs`
- Modify: `voxel_game/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `voxel_game/src/inventory/mod.rs`:

```rust
pub mod crafting;
pub mod ui;

use bevy::prelude::*;
use crate::types::{VoxelId, AIR};

pub const INVENTORY_SIZE: usize = 36;
pub const HOTBAR_SIZE: usize = 9;

#[derive(Debug, Clone, Copy, Default)]
pub struct InventorySlot {
    pub voxel_id: VoxelId,
    pub count: u16,
}

#[derive(Resource)]
pub struct Inventory {
    pub slots: [InventorySlot; INVENTORY_SIZE],
    pub active_slot: usize,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: [InventorySlot::default(); INVENTORY_SIZE],
            active_slot: 0,
        }
    }
}

impl Inventory {
    pub fn add(&mut self, voxel_id: VoxelId, count: u16) -> u16 {
        // Add to existing stack first, then empty slot
        todo!()
    }

    pub fn remove(&mut self, slot: usize, count: u16) -> bool {
        // Returns true if successful
        todo!()
    }

    pub fn active_voxel_id(&self) -> VoxelId {
        self.slots[self.active_slot].voxel_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::STONE;

    #[test]
    fn add_to_empty_inventory() {
        let mut inv = Inventory::default();
        let leftover = inv.add(STONE, 5);
        assert_eq!(leftover, 0);
        assert_eq!(inv.slots[0].voxel_id, STONE);
        assert_eq!(inv.slots[0].count, 5);
    }

    #[test]
    fn add_stacks_with_existing() {
        let mut inv = Inventory::default();
        inv.add(STONE, 5);
        inv.add(STONE, 3);
        assert_eq!(inv.slots[0].count, 8, "should stack into same slot");
    }

    #[test]
    fn remove_decrements_count() {
        let mut inv = Inventory::default();
        inv.add(STONE, 5);
        let ok = inv.remove(0, 3);
        assert!(ok);
        assert_eq!(inv.slots[0].count, 2);
    }

    #[test]
    fn remove_clears_empty_slot() {
        let mut inv = Inventory::default();
        inv.add(STONE, 3);
        inv.remove(0, 3);
        assert_eq!(inv.slots[0].voxel_id, AIR);
        assert_eq!(inv.slots[0].count, 0);
    }

    #[test]
    fn remove_fails_when_insufficient() {
        let mut inv = Inventory::default();
        inv.add(STONE, 2);
        let ok = inv.remove(0, 5);
        assert!(!ok);
        assert_eq!(inv.slots[0].count, 2, "count unchanged on failure");
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test -p voxel_game inventory::tests
```

Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Implement Inventory methods**

Replace the `todo!()` in `voxel_game/src/inventory/mod.rs`:

```rust
impl Inventory {
    pub fn add(&mut self, voxel_id: VoxelId, count: u16) -> u16 {
        // Try to stack with existing slot of same type
        for slot in &mut self.slots {
            if slot.voxel_id == voxel_id {
                slot.count = slot.count.saturating_add(count);
                return 0;
            }
        }
        // Place in first empty slot
        for slot in &mut self.slots {
            if slot.voxel_id == AIR {
                slot.voxel_id = voxel_id;
                slot.count = count;
                return 0;
            }
        }
        count // inventory full — return leftover
    }

    pub fn remove(&mut self, slot: usize, count: u16) -> bool {
        if self.slots[slot].count < count {
            return false;
        }
        self.slots[slot].count -= count;
        if self.slots[slot].count == 0 {
            self.slots[slot].voxel_id = AIR;
        }
        true
    }

    pub fn active_voxel_id(&self) -> VoxelId {
        self.slots[self.active_slot].voxel_id
    }
}
```

Also add stub files:
```bash
touch voxel_game/src/inventory/crafting.rs
touch voxel_game/src/inventory/ui.rs
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p voxel_game inventory::tests
```

Expected: all 5 tests PASS

- [ ] **Step 5: Register in lib.rs**

Add to `voxel_game/src/lib.rs`:
```rust
pub mod inventory;
```

And in `VoxelGamePlugin::build`:
```rust
app.init_resource::<inventory::Inventory>();
```

- [ ] **Step 6: Commit**

```bash
git add voxel_game/src/inventory/
git commit -m "feat(voxel_game): inventory slot system"
```

---

## Task 4: Voxel Break + Debris Ejection

**Files:**
- Create: `voxel_game/src/simulation/mod.rs`
- Create: `voxel_game/src/simulation/debris.rs`
- Modify: `voxel_game/src/player/interaction.rs`
- Modify: `voxel_game/src/lib.rs`

- [ ] **Step 1: Create simulation module stubs**

Create `voxel_game/src/simulation/mod.rs`:

```rust
pub mod debris;

use bevy::prelude::*;

pub struct SimulationPlugin;
impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            debris::tick_debris,
            debris::solidify_resting_debris,
        ));
    }
}
```

Create `voxel_game/src/simulation/debris.rs`:

```rust
use bevy::prelude::*;
use crate::chunk::loading::ChunkedWorld;
use crate::config::VOXEL_SIZE;
use crate::types::{ChunkPos, LocalVoxelPos, VoxelId};

const GRAVITY: f32 = -9.8;
const SOLIDIFY_SECS: f32 = 5.0;
const PICKUP_RADIUS: f32 = 1.5;

#[derive(Component)]
pub struct DebrisParticle {
    pub voxel_id: VoxelId,
    pub velocity: Vec3,
    pub rest_timer: f32, // seconds at rest; reaches SOLIDIFY_SECS → solidify
}

impl DebrisParticle {
    pub fn new(voxel_id: VoxelId, velocity: Vec3) -> Self {
        Self { voxel_id, velocity, rest_timer: 0.0 }
    }
}

pub fn spawn_debris(
    commands: &mut Commands,
    voxel_id: VoxelId,
    world_pos: Vec3,
    velocity: Vec3,
) {
    commands.spawn((
        DebrisParticle::new(voxel_id, velocity),
        TransformBundle::from(Transform::from_translation(world_pos)),
        Visibility::Visible,
    ));
}

pub fn tick_debris(
    mut commands: Commands,
    mut debris_query: Query<(Entity, &mut Transform, &mut DebrisParticle)>,
    world: Res<ChunkedWorld>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (_entity, mut transform, mut debris) in &mut debris_query {
        // Apply gravity
        debris.velocity.y += GRAVITY * dt;

        let next_pos = transform.translation + debris.velocity * dt;

        // Check if the voxel below is solid
        let below = next_pos - Vec3::Y * VOXEL_SIZE;
        let floor_solid = is_world_pos_solid(&world, below);

        if floor_solid && debris.velocity.y < 0.0 {
            debris.velocity.y = 0.0;
            debris.velocity.x *= 0.8;
            debris.velocity.z *= 0.8;
        }

        // Check horizontal (simplified: just stop on solid)
        let next_x = Vec3::new(next_pos.x, transform.translation.y, transform.translation.z);
        if is_world_pos_solid(&world, next_x) {
            debris.velocity.x = 0.0;
        }
        let next_z = Vec3::new(transform.translation.x, transform.translation.y, next_pos.z);
        if is_world_pos_solid(&world, next_z) {
            debris.velocity.z = 0.0;
        }

        let speed = debris.velocity.length();
        if speed < 0.05 {
            debris.velocity = Vec3::ZERO;
            debris.rest_timer += dt;
        } else {
            debris.rest_timer = 0.0;
        }

        transform.translation += debris.velocity * dt;
    }
}

pub fn solidify_resting_debris(
    mut commands: Commands,
    debris_query: Query<(Entity, &Transform, &DebrisParticle)>,
    mut world: ResMut<ChunkedWorld>,
) {
    for (entity, transform, debris) in &debris_query {
        if debris.rest_timer < SOLIDIFY_SECS { continue; }

        let pos = transform.translation;
        let n = crate::config::CHUNK_SIZE as i32;
        let vx = (pos.x / VOXEL_SIZE).floor() as i32;
        let vy = (pos.y / VOXEL_SIZE).floor() as i32;
        let vz = (pos.z / VOXEL_SIZE).floor() as i32;

        let chunk_pos = ChunkPos(vx.div_euclid(n), vy.div_euclid(n), vz.div_euclid(n));
        let local = LocalVoxelPos::new(
            vx.rem_euclid(n) as u8,
            vy.rem_euclid(n) as u8,
            vz.rem_euclid(n) as u8,
        );

        if let Some(chunk) = world.get_mut(chunk_pos) {
            chunk.set(local, debris.voxel_id); // marks chunk dirty → re-meshes
        }

        commands.entity(entity).despawn();
    }
}

fn is_world_pos_solid(world: &ChunkedWorld, pos: Vec3) -> bool {
    let n = crate::config::CHUNK_SIZE as i32;
    let vx = (pos.x / VOXEL_SIZE).floor() as i32;
    let vy = (pos.y / VOXEL_SIZE).floor() as i32;
    let vz = (pos.z / VOXEL_SIZE).floor() as i32;
    let chunk_pos = ChunkPos(vx.div_euclid(n), vy.div_euclid(n), vz.div_euclid(n));
    let local = LocalVoxelPos::new(
        vx.rem_euclid(n) as u8,
        vy.rem_euclid(n) as u8,
        vz.rem_euclid(n) as u8,
    );
    world.get(chunk_pos).map_or(false, |c| c.is_solid(local))
}
```

- [ ] **Step 2: Add break input system to interaction.rs**

Append to `voxel_game/src/player/interaction.rs`:

```rust
use crate::simulation::debris::spawn_debris;
use crate::inventory::Inventory;
use crate::game_mode::GameMode;

pub fn handle_break_place(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    targeted: Res<TargetedVoxel>,
    mut world: ResMut<ChunkedWorld>,
    camera_query: Query<&Transform, With<crate::player::camera::PlayerCamera>>,
    mut inventory: ResMut<Inventory>,
    game_mode: Res<GameMode>,
) {
    // Left click: break
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(hit) = &targeted.0 {
            if let Some(chunk) = world.get_mut(hit.chunk) {
                let voxel_id = chunk.get(hit.local);
                chunk.set(hit.local, crate::types::AIR);
                // Eject debris at the voxel center
                let voxel_world = hit.chunk.to_world_origin()
                    + hit.local.to_local_world()
                    + Vec3::splat(VOXEL_SIZE * 0.5);
                let eject_vel = Vec3::new(0.0, 2.0, 0.0); // pop upward
                spawn_debris(&mut commands, voxel_id, voxel_world, eject_vel);
            }
        }
    }

    // Right click: place
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(hit) = &targeted.0 {
            let place_voxel_id = inventory.active_voxel_id();
            if place_voxel_id == crate::types::AIR { return; }

            // Place on the face normal side of the hit voxel
            let n = crate::config::CHUNK_SIZE as i32;
            let voxel_ivec = IVec3::new(
                hit.chunk.0 * n + hit.local.x as i32,
                hit.chunk.1 * n + hit.local.y as i32,
                hit.chunk.2 * n + hit.local.z as i32,
            ) + hit.normal;

            let place_chunk = ChunkPos(
                voxel_ivec.x.div_euclid(n),
                voxel_ivec.y.div_euclid(n),
                voxel_ivec.z.div_euclid(n),
            );
            let place_local = LocalVoxelPos::new(
                voxel_ivec.x.rem_euclid(n) as u8,
                voxel_ivec.y.rem_euclid(n) as u8,
                voxel_ivec.z.rem_euclid(n) as u8,
            );

            if let Some(chunk) = world.get_mut(place_chunk) {
                let creative = *game_mode == GameMode::Creative;
                let can_place = creative || inventory.remove(inventory.active_slot, 1);
                if can_place {
                    chunk.set(place_local, place_voxel_id);
                }
            }
        }
    }
}

pub fn handle_pickup(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, With<crate::player::camera::PlayerCamera>>,
    debris_query: Query<(Entity, &Transform, &crate::simulation::debris::DebrisParticle)>,
    mut inventory: ResMut<Inventory>,
) {
    if !keys.just_pressed(KeyCode::KeyF) { return; }
    let Ok(cam) = camera_query.get_single() else { return };

    for (entity, transform, debris) in &debris_query {
        if transform.translation.distance(cam.translation) <= REACH {
            inventory.add(debris.voxel_id, 1);
            commands.entity(entity).despawn();
        }
    }
}
```

- [ ] **Step 3: Register systems**

In `voxel_game/src/player/mod.rs`, add to `PlayerPlugin::build`:

```rust
.add_systems(Update, (
    interaction::handle_break_place,
    interaction::handle_pickup,
));
```

In `voxel_game/src/lib.rs`, add:
```rust
pub mod simulation;
```

And in `VoxelGamePlugin::build`:
```rust
.add_plugins(simulation::SimulationPlugin)
```

- [ ] **Step 4: Write integration test for break → debris cycle**

Create `voxel_game/tests/debris_integration.rs`:

```rust
use bevy::prelude::*;
use voxel_game::chunk::Chunk;
use voxel_game::chunk::loading::ChunkedWorld;
use voxel_game::simulation::debris::{DebrisParticle, tick_debris, solidify_resting_debris};
use voxel_game::types::{ChunkPos, LocalVoxelPos, STONE, AIR};
use voxel_game::config::VOXEL_SIZE;

#[test]
fn debris_falls_and_stops() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ChunkedWorld>();

    // Spawn floor (stone at y=0)
    {
        let mut world = app.world_mut().resource_mut::<ChunkedWorld>();
        let mut floor_chunk = Chunk::new();
        for x in 0..voxel_game::config::CHUNK_SIZE {
            for z in 0..voxel_game::config::CHUNK_SIZE {
                floor_chunk.set(LocalVoxelPos::new(x as u8, 0, z as u8), STONE);
            }
        }
        world.chunks.insert(ChunkPos(0, 0, 0), floor_chunk);
    }

    // Spawn debris above floor
    let debris_id = app.world_mut().spawn((
        DebrisParticle::new(STONE, Vec3::ZERO),
        Transform::from_xyz(0.5, 2.0, 0.5),
    )).id();

    app.add_systems(Update, tick_debris);

    // Run enough frames for debris to fall to y≈0.1 (one voxel above floor)
    for _ in 0..120 {
        app.update();
    }

    let transform = app.world().entity(debris_id).get::<Transform>().unwrap();
    assert!(transform.translation.y < 0.5, "debris should have fallen");
    assert!(transform.translation.y > 0.0, "debris should rest above floor, not clip through");
}

#[test]
fn debris_solidifies_into_chunk() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ChunkedWorld>();

    // Empty chunk at origin
    {
        let mut world = app.world_mut().resource_mut::<ChunkedWorld>();
        world.chunks.insert(ChunkPos(0, 0, 0), Chunk::new());
    }

    // Spawn debris already at rest with timer just under threshold
    let _debris_id = app.world_mut().spawn((
        DebrisParticle {
            voxel_id: STONE,
            velocity: Vec3::ZERO,
            rest_timer: 4.9,
        },
        Transform::from_xyz(0.55, 0.55, 0.55), // inside voxel (5,5,5)
    )).id();

    app.add_systems(Update, (tick_debris, solidify_resting_debris));

    // Run until solidify triggers (~1-2 frames at 60fps to push timer over 5s)
    for _ in 0..10 {
        app.update();
    }

    let world = app.world().resource::<ChunkedWorld>();
    let chunk = world.get(ChunkPos(0, 0, 0)).expect("chunk should exist");
    let local = LocalVoxelPos::new(5, 5, 5);
    assert_eq!(chunk.get(local), STONE, "debris should have solidified into chunk");
}
```

- [ ] **Step 5: Run integration tests**

```bash
cargo test -p voxel_game --test debris_integration
```

Expected: both tests PASS

- [ ] **Step 6: Commit**

```bash
git add voxel_game/src/simulation/ voxel_game/src/player/interaction.rs \
        voxel_game/tests/debris_integration.rs voxel_game/src/lib.rs
git commit -m "feat(voxel_game): debris CA simulation, break/place/pickup input"
```

---

## Task 5: Tools

**Files:**
- Modify: `voxel_game/src/inventory/mod.rs`
- Modify: `voxel_game/src/player/interaction.rs`

- [ ] **Step 1: Write failing test for tool break eligibility**

Add to `voxel_game/src/inventory/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Hand,
    Pickaxe,
    Shovel,
}

impl ToolType {
    /// Whether this tool can break `voxel_id`
    pub fn can_break(self, voxel_id: VoxelId) -> bool { todo!() }
    /// Debris particles ejected per swing
    pub fn debris_count(self) -> u8 { todo!() }
}
```

Add test:

```rust
#[cfg(test)]
mod tool_tests {
    use super::*;
    use crate::types::{STONE, DIRT, TOPSOIL};

    #[test]
    fn pickaxe_can_break_stone() {
        assert!(ToolType::Pickaxe.can_break(STONE));
    }

    #[test]
    fn hand_cannot_break_stone() {
        assert!(!ToolType::Hand.can_break(STONE));
    }

    #[test]
    fn hand_can_break_dirt() {
        assert!(ToolType::Hand.can_break(DIRT));
        assert!(ToolType::Hand.can_break(TOPSOIL));
    }

    #[test]
    fn pickaxe_ejects_three_debris() {
        assert_eq!(ToolType::Pickaxe.debris_count(), 3);
    }

    #[test]
    fn hand_ejects_one_debris() {
        assert_eq!(ToolType::Hand.debris_count(), 1);
    }
}
```

- [ ] **Step 2: Run tool tests — expect fail**

```bash
cargo test -p voxel_game tool_tests
```

Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Implement ToolType**

Replace the `todo!()` in `voxel_game/src/inventory/mod.rs`:

```rust
impl ToolType {
    pub fn can_break(self, voxel_id: VoxelId) -> bool {
        match self {
            ToolType::Hand => voxel_id == crate::types::DIRT || voxel_id == crate::types::TOPSOIL,
            ToolType::Pickaxe => true, // pickaxe can break everything
            ToolType::Shovel => voxel_id == crate::types::DIRT || voxel_id == crate::types::TOPSOIL,
        }
    }

    pub fn debris_count(self) -> u8 {
        match self {
            ToolType::Hand => 1,
            ToolType::Pickaxe => 3,
            ToolType::Shovel => 2,
        }
    }
}
```

Add an `active_tool` field to `Inventory`:

```rust
pub struct Inventory {
    pub slots: [InventorySlot; INVENTORY_SIZE],
    pub active_slot: usize,
    pub active_tool: ToolType,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: [InventorySlot::default(); INVENTORY_SIZE],
            active_slot: 0,
            active_tool: ToolType::Hand,
        }
    }
}
```

- [ ] **Step 4: Gate breaking on tool eligibility**

In `voxel_game/src/player/interaction.rs`, update the left-click branch in `handle_break_place` to check tool eligibility and eject multiple debris:

```rust
if mouse.just_pressed(MouseButton::Left) {
    if let Some(hit) = &targeted.0 {
        let tool = inventory.active_tool;
        if !tool.can_break(hit.voxel_id) { return; }

        if let Some(chunk) = world.get_mut(hit.chunk) {
            let voxel_id = chunk.get(hit.local);
            chunk.set(hit.local, crate::types::AIR);
            let voxel_world = hit.chunk.to_world_origin()
                + hit.local.to_local_world()
                + Vec3::splat(VOXEL_SIZE * 0.5);

            // Eject `debris_count` particles with slight random spread
            for i in 0..tool.debris_count() {
                let spread = Vec3::new(
                    (i as f32 * 0.3) - 0.3,
                    2.0 + i as f32 * 0.2,
                    (i as f32 * 0.2) - 0.2,
                );
                spawn_debris(&mut commands, voxel_id, voxel_world, spread);
            }
        }
    }
}
```

- [ ] **Step 5: Run tool tests — expect pass**

```bash
cargo test -p voxel_game tool_tests
```

Expected: all 5 tests PASS

- [ ] **Step 6: Commit**

```bash
git add voxel_game/src/inventory/mod.rs voxel_game/src/player/interaction.rs
git commit -m "feat(voxel_game): tool types and break eligibility"
```

---

## Task 6: Crafting

**Files:**
- Create: `voxel_game/src/inventory/crafting.rs`
- Create: `voxel_game/assets/recipes.ron`
- Modify: `voxel_game/src/lib.rs`

- [ ] **Step 1: Write failing crafting tests**

Create `voxel_game/src/inventory/crafting.rs`:

```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::types::VoxelId;
use super::Inventory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub inputs: Vec<(VoxelId, u16)>,
    pub output: (VoxelId, u16),
}

#[derive(Resource, Default)]
pub struct RecipeBook(pub Vec<Recipe>);

impl RecipeBook {
    /// Returns recipes the player can currently craft given their inventory.
    pub fn available(&self, inventory: &Inventory) -> Vec<&Recipe> {
        self.0.iter().filter(|r| can_craft(inventory, r)).collect()
    }
}

pub fn can_craft(inventory: &Inventory, recipe: &Recipe) -> bool {
    recipe.inputs.iter().all(|(voxel_id, needed)| {
        let have: u16 = inventory.slots.iter()
            .filter(|s| s.voxel_id == *voxel_id)
            .map(|s| s.count)
            .sum();
        have >= *needed
    })
}

/// Consume inputs and produce output. Returns false if ingredients are missing.
pub fn apply_craft(inventory: &mut Inventory, recipe: &Recipe) -> bool {
    if !can_craft(inventory, recipe) { return false; }
    for &(voxel_id, count) in &recipe.inputs {
        let mut to_remove = count;
        for slot in &mut inventory.slots {
            if slot.voxel_id == voxel_id && to_remove > 0 {
                let take = to_remove.min(slot.count);
                slot.count -= take;
                to_remove -= take;
                if slot.count == 0 { slot.voxel_id = crate::types::AIR; }
            }
        }
    }
    inventory.add(recipe.output.0, recipe.output.1);
    true
}

pub fn load_recipes(mut recipe_book: ResMut<RecipeBook>, asset_server: Res<AssetServer>) {
    // Inline default recipes for now; file loading added in a later task
    recipe_book.0 = vec![
        Recipe {
            inputs: vec![(crate::types::STONE, 4)],
            output: (crate::types::STONE, 4), // placeholder — real recipes added in assets/recipes.ron
        },
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{STONE, DIRT};
    use crate::inventory::Inventory;

    fn pickaxe_recipe() -> Recipe {
        Recipe {
            inputs: vec![(STONE, 3)],
            output: (STONE, 1), // simplified output for test
        }
    }

    #[test]
    fn can_craft_when_enough_ingredients() {
        let mut inv = Inventory::default();
        inv.add(STONE, 5);
        assert!(can_craft(&inv, &pickaxe_recipe()));
    }

    #[test]
    fn cannot_craft_when_insufficient() {
        let mut inv = Inventory::default();
        inv.add(STONE, 2);
        assert!(!can_craft(&inv, &pickaxe_recipe()));
    }

    #[test]
    fn apply_craft_consumes_inputs() {
        let mut inv = Inventory::default();
        inv.add(STONE, 5);
        let ok = apply_craft(&mut inv, &pickaxe_recipe());
        assert!(ok);
        // 5 stone consumed 3, +1 output = 3 stone total
        let stone_count: u16 = inv.slots.iter().filter(|s| s.voxel_id == STONE).map(|s| s.count).sum();
        assert_eq!(stone_count, 3);
    }

    #[test]
    fn apply_craft_fails_when_insufficient() {
        let mut inv = Inventory::default();
        inv.add(STONE, 1);
        let ok = apply_craft(&mut inv, &pickaxe_recipe());
        assert!(!ok);
        let stone_count: u16 = inv.slots.iter().filter(|s| s.voxel_id == STONE).map(|s| s.count).sum();
        assert_eq!(stone_count, 1, "inventory unchanged on failure");
    }
}
```

- [ ] **Step 2: Run crafting tests — expect fail**

```bash
cargo test -p voxel_game crafting
```

Expected: FAIL (module not yet registered)

- [ ] **Step 3: Register RecipeBook resource**

In `voxel_game/src/lib.rs`, add to `VoxelGamePlugin::build`:

```rust
.init_resource::<inventory::crafting::RecipeBook>()
.add_systems(Startup, inventory::crafting::load_recipes)
```

- [ ] **Step 4: Run crafting tests — expect pass**

```bash
cargo test -p voxel_game crafting
```

Expected: all 4 tests PASS

- [ ] **Step 5: Create assets directory and recipes stub**

```bash
mkdir -p voxel_game/assets
```

Create `voxel_game/assets/recipes.ron`:

```ron
// Recipe format: inputs (voxel_id, count) -> output (voxel_id, count)
// VoxelId values: 0=AIR, 1=STONE, 2=DIRT, 3=TOPSOIL
[
    Recipe(
        inputs: [(1, 4)],
        output: (1, 4),
    ),
]
```

- [ ] **Step 6: Commit**

```bash
git add voxel_game/src/inventory/crafting.rs voxel_game/assets/ voxel_game/src/lib.rs
git commit -m "feat(voxel_game): recipe crafting system"
```

---

## Task 7: Hotbar UI

**Files:**
- Create: `voxel_game/src/inventory/ui.rs`
- Modify: `voxel_game/src/lib.rs`

- [ ] **Step 1: Create hotbar HUD**

Create `voxel_game/src/inventory/ui.rs`:

```rust
use bevy::prelude::*;
use crate::inventory::{Inventory, HOTBAR_SIZE};
use crate::types::AIR;

#[derive(Component)]
pub struct HotbarSlotUi(pub usize);

pub fn spawn_hotbar(mut commands: Commands) {
    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            left: Val::Percent(50.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            ..default()
        },
        ..default()
    }).with_children(|parent| {
        for i in 0..HOTBAR_SIZE {
            parent.spawn((
                HotbarSlotUi(i),
                NodeBundle {
                    style: Style {
                        width: Val::Px(48.0),
                        height: Val::Px(48.0),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.7)),
                    border_color: BorderColor(Color::srgb(0.4, 0.4, 0.4)),
                    ..default()
                },
            ));
        }
    });
}

pub fn update_hotbar(
    inventory: Res<Inventory>,
    mut slot_query: Query<(&HotbarSlotUi, &mut BorderColor)>,
) {
    for (slot_ui, mut border) in &mut slot_query {
        border.0 = if slot_ui.0 == inventory.active_slot {
            Color::srgb(1.0, 1.0, 0.0) // yellow highlight
        } else {
            Color::srgb(0.4, 0.4, 0.4)
        };
    }
}

pub fn cycle_hotbar(
    mut inventory: ResMut<Inventory>,
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
) {
    // Number keys 1-9
    let number_keys = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
        KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
        KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
    ];
    for (i, key) in number_keys.iter().enumerate() {
        if keys.just_pressed(*key) {
            inventory.active_slot = i;
        }
    }

    // Scroll wheel
    for event in scroll.read() {
        let delta = if event.y > 0.0 { -1i32 } else { 1 };
        let new = (inventory.active_slot as i32 + delta)
            .rem_euclid(HOTBAR_SIZE as i32) as usize;
        inventory.active_slot = new;
    }
}
```

- [ ] **Step 2: Register UI systems**

In `voxel_game/src/lib.rs`, add to `VoxelGamePlugin::build`:

```rust
.add_systems(Startup, inventory::ui::spawn_hotbar)
.add_systems(Update, (
    inventory::ui::update_hotbar,
    inventory::ui::cycle_hotbar,
))
```

- [ ] **Step 3: Run and verify hotbar appears**

```bash
cargo run -p voxel_game
```

Expected: a row of 9 grey squares appears at the bottom center of the screen. The active slot is highlighted yellow. Scrolling or pressing 1-9 changes the highlight.

- [ ] **Step 4: Commit**

```bash
git add voxel_game/src/inventory/ui.rs voxel_game/src/lib.rs
git commit -m "feat(voxel_game): hotbar UI with scroll and number key selection"
```

---

## Task 8: ProceduralGenerator

**Files:**
- Create: `voxel_game/src/world/procedural.rs`
- Create: `voxel_game/benches/generation.rs`
- Modify: `voxel_game/src/world/mod.rs`

- [ ] **Step 1: Write failing generation tests**

Create `voxel_game/src/world/procedural.rs`:

```rust
use noise::{NoiseFn, Perlin, Fbm, MultiFractal};
use crate::chunk::Chunk;
use crate::config::{CHUNK_SIZE, VOXEL_SIZE};
use crate::types::{ChunkPos, LocalVoxelPos, VoxelId, AIR, STONE, DIRT, TOPSOIL};
use super::WorldGenerator;

pub struct ProceduralGenerator {
    pub seed: u64,
    pub surface_scale: f64,   // horizontal noise scale (larger = broader hills)
    pub surface_amplitude: f64, // height variation in voxels
    pub surface_base_y: f64,  // base surface voxel Y
    pub cave_threshold: f64,  // 0.0-1.0; higher = fewer caves
}

impl ProceduralGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            surface_scale: 0.005,
            surface_amplitude: 40.0,
            surface_base_y: 0.0,
            cave_threshold: 0.65,
        }
    }
}

impl WorldGenerator for ProceduralGenerator {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk { todo!() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AIR;

    fn gen() -> ProceduralGenerator { ProceduralGenerator::new(42) }

    #[test]
    fn high_chunk_is_mostly_air() {
        let g = gen();
        // A chunk very high up should be almost entirely air
        let chunk = g.generate_chunk(ChunkPos(0, 20, 0));
        let air_count = chunk.voxels.iter().filter(|&&v| v == AIR).count();
        let total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
        assert!(air_count > total * 9 / 10, "high chunk should be >90% air");
    }

    #[test]
    fn deep_chunk_is_mostly_solid() {
        let g = gen();
        // A chunk far underground should be mostly solid
        let chunk = g.generate_chunk(ChunkPos(0, -10, 0));
        let solid_count = chunk.voxels.iter().filter(|&&v| v != AIR).count();
        let total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
        assert!(solid_count > total / 2, "deep chunk should be >50% solid");
    }

    #[test]
    fn surface_chunk_has_topsoil_on_top() {
        let g = gen();
        // Chunk at surface level should have topsoil voxels somewhere
        let chunk = g.generate_chunk(ChunkPos(0, 0, 0));
        let has_topsoil = chunk.voxels.iter().any(|&v| v == TOPSOIL);
        // Surface might be in a different chunk depending on noise, so just check it has mixed content
        let has_air = chunk.voxels.iter().any(|&v| v == AIR);
        let has_solid = chunk.voxels.iter().any(|&v| v != AIR);
        assert!(has_air && has_solid, "surface chunk should have mixed air and solid");
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

```bash
cargo test -p voxel_game procedural
```

Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Implement ProceduralGenerator**

Replace the `todo!()` in `voxel_game/src/world/procedural.rs`:

```rust
impl WorldGenerator for ProceduralGenerator {
    fn generate_chunk(&self, pos: ChunkPos) -> Chunk {
        let n = CHUNK_SIZE as i32;
        let seed = self.seed as u32;
        let surface_noise: Fbm<Perlin> = Fbm::new(seed)
            .set_octaves(4)
            .set_frequency(self.surface_scale);
        let cave_noise = Perlin::new(seed.wrapping_add(1));

        let mut chunk = Chunk::new();

        for lz in 0..n as u8 {
            for lx in 0..n as u8 {
                // World X and Z for this column
                let wx = pos.0 * n + lx as i32;
                let wz = pos.2 * n + lz as i32;

                // Sample 2D surface height (in world voxels)
                let nx = wx as f64 * VOXEL_SIZE as f64;
                let nz = wz as f64 * VOXEL_SIZE as f64;
                let height_offset = surface_noise.get([nx, nz]) * self.surface_amplitude;
                let surface_voxel_y = (self.surface_base_y + height_offset) as i32;

                for ly in 0..n as u8 {
                    let wy = pos.1 * n + ly as i32; // world voxel Y

                    let voxel = if wy > surface_voxel_y {
                        AIR
                    } else if wy == surface_voxel_y {
                        TOPSOIL
                    } else if wy > surface_voxel_y - 3 {
                        DIRT
                    } else {
                        // Cave carving: 3D Perlin
                        let cv = cave_noise.get([
                            wx as f64 * VOXEL_SIZE as f64 * 0.5,
                            wy as f64 * VOXEL_SIZE as f64 * 0.5,
                            wz as f64 * VOXEL_SIZE as f64 * 0.5,
                        ]);
                        if cv.abs() > self.cave_threshold {
                            AIR // cave
                        } else {
                            STONE
                        }
                    };

                    chunk.set(LocalVoxelPos::new(lx, ly, lz), voxel);
                }
            }
        }

        chunk
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p voxel_game procedural
```

Expected: all 3 tests PASS

- [ ] **Step 5: Create generation benchmark**

Create `voxel_game/benches/generation.rs`:

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use voxel_game::types::ChunkPos;
use voxel_game::world::WorldGenerator;
use voxel_game::world::flat::FlatGenerator;
use voxel_game::world::procedural::ProceduralGenerator;
use voxel_game::types::STONE;

fn bench_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_generation");

    let flat = FlatGenerator::new(0, STONE);
    let proc = ProceduralGenerator::new(42);
    let pos = ChunkPos(3, -1, 7);

    group.bench_with_input(BenchmarkId::new("FlatGenerator", ""), &pos,
        |b, &p| b.iter(|| flat.generate_chunk(p)));

    group.bench_with_input(BenchmarkId::new("ProceduralGenerator", ""), &pos,
        |b, &p| b.iter(|| proc.generate_chunk(p)));

    group.finish();
}

criterion_group!(benches, bench_generation);
criterion_main!(benches);
```

Add to `voxel_game/Cargo.toml`:
```toml
[[bench]]
name = "generation"
harness = false
```

- [ ] **Step 6: Run benchmark to confirm it compiles**

```bash
cargo bench -p voxel_game --bench generation -- --test
```

Expected: "test generation::bench_generation ... ok"

- [ ] **Step 7: Register procedural generator module**

In `voxel_game/src/world/mod.rs`, add:
```rust
pub mod procedural;
```

- [ ] **Step 8: Commit**

```bash
git add voxel_game/src/world/procedural.rs voxel_game/benches/generation.rs voxel_game/Cargo.toml
git commit -m "feat(voxel_game): ProceduralGenerator with surface, caves, and ore layers"
```

---

## Task 9: Switch to Procedural World & Final Polish

**Files:**
- Modify: `voxel_game/src/lib.rs`

- [ ] **Step 1: Swap default generator to ProceduralGenerator**

Update `VoxelGamePlugin::build` in `voxel_game/src/lib.rs`:

```rust
use world::procedural::ProceduralGenerator;

// Replace FlatGenerator line with:
.insert_resource(ActiveWorldGenerator(Box::new(
    ProceduralGenerator::new(12345),
)))
```

- [ ] **Step 2: Run and verify procedural world**

```bash
cargo run -p voxel_game
```

Expected: hilly terrain with visible stone, dirt layers, and cave openings. Player spawns and falls to the surface. Breaking stone with the default hand will fail (hand can't break stone — add a pickaxe to creative inventory to test, or temporarily set `active_tool: ToolType::Pickaxe` in the `Inventory::default()`).

- [ ] **Step 3: Run full test suite**

```bash
cargo test -p voxel_game
```

Expected: all tests pass

- [ ] **Step 4: Run all benchmarks**

```bash
cargo bench -p voxel_game
```

Expected: HTML reports at `target/criterion/greedy_mesh/` and `target/criterion/chunk_generation/`

- [ ] **Step 5: Revert to FlatGenerator for default build** (optional — keep sandbox as default while in development)

This is a preference call. If you want the procedural world as default, keep it. If you want the flat sandbox for easier development, revert:

```rust
.insert_resource(ActiveWorldGenerator(Box::new(
    FlatGenerator::new(0, STONE),
)))
```

- [ ] **Step 6: Final commit**

```bash
git add -p
git commit -m "feat(voxel_game): phase 2 complete - gameplay mechanics"
```

---

## Phase 2 Complete

At this point you have:
- Voxel targeting with DDA raycast
- Break/place with tool eligibility
- Falling debris CA simulation (solidification + pickup)
- Inventory with stacking and hotbar UI
- Recipe crafting system
- ProceduralGenerator with terrain, caves, and material layering
- All tests passing, benchmarks running

**Out of scope (future sessions):** persistence/save-load, structural integrity, enemies, biomes, multiplayer.
