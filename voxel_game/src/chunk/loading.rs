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
        let generator_arc = generator.0.clone();
        let task = task_pool.spawn(async move { generator_arc.generate_chunk(pos) });
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
