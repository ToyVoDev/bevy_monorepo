use bevy::prelude::*;
use crate::ui::screens::Screen;
use crate::ui::theme::widget;
use crate::chunk::loading::{ChunkedWorld, PendingGeneration};
use crate::types::ChunkPos;
use crate::Settings;

const MIN_DISPLAY_SECS: f32 = 3.0;

#[derive(Resource)]
struct WorldLoadingTimer(Timer);

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
    world: Res<ChunkedWorld>,
) {
    info!("WorldLoading: entered (spawn_radius={}, render_distance={})",
        settings.spawn_radius, settings.render_distance);

    commands.spawn((
        widget::ui_root("World Loading Screen"),
        DespawnOnExit(Screen::WorldLoading),
        children![widget::label("Generating world...")],
    ));

    // Queue the full render_distance cube so the 3-second window generates as many
    // chunks as possible. Surface-first sort ensures spawn_radius chunks come first.
    let r = settings.render_distance as i32;
    pending.0.clear();
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                let pos = ChunkPos(dx, dy, dz);
                if !world.chunks.contains_key(&pos) {
                    pending.0.push_back(pos);
                }
            }
        }
    }
    // Spawn is always at origin, so relative distance == absolute distance.
    pending.0.make_contiguous().sort_unstable_by_key(|p| {
        let xz = (p.0.abs() + p.2.abs()) as i64;
        let dy = p.1 as i64;
        let y_cost = if dy < 0 { (-dy) * 4 } else { dy };
        xz + y_cost
    });

    info!("WorldLoading: queued {} chunks to generate", pending.0.len());
    commands.insert_resource(WorldLoadingTimer(Timer::from_seconds(
        MIN_DISPLAY_SECS,
        TimerMode::Once,
    )));
}

fn check_spawn_ready(
    world: Res<ChunkedWorld>,
    settings: Res<Settings>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut timer: ResMut<WorldLoadingTimer>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());

    let spawn_r = settings.spawn_radius as i32;
    let render_r = settings.render_distance as i32;
    let total = ((render_r * 2 + 1) as usize).pow(3);
    let loaded = world.chunks.len().min(total);

    let spawn_ready = all_spawn_chunks_present(&world, settings.spawn_radius);
    if !timer.0.is_finished() || !spawn_ready {
        // Log progress once per second
        let elapsed = timer.0.elapsed_secs();
        if (elapsed * 2.0) as u32 != ((elapsed * 2.0 - time.delta_secs() * 2.0) as u32) {
            let spawn_total = ((spawn_r * 2 + 1) as usize).pow(3);
            let spawn_loaded = world.chunks.keys()
                .filter(|p| p.0.abs() <= spawn_r && p.1.abs() <= spawn_r && p.2.abs() <= spawn_r)
                .count();
            info!("WorldLoading: {}/{} total chunks | {}/{} spawn chunks | {:.1}s elapsed",
                loaded, total, spawn_loaded, spawn_total, elapsed);
        }
        return;
    }

    info!("WorldLoading: {} chunks loaded, spawn area ready — transitioning to Gameplay", loaded);
    next_screen.set(Screen::Gameplay);
}

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
