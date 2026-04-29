use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use crate::chunk::Chunk;
use crate::types::ChunkPos;
use crate::world::ActiveWorldGenerator;

pub const LOAD_RADIUS: i32 = 10;
pub const MAX_CHUNKS_PER_FRAME: usize = 8;
pub const HARD_RADIUS: i32 = 1;

#[derive(Resource, Default)]
pub struct PriorityMeshQueue(pub Vec<crate::types::ChunkPos>);

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
    generator: Res<ActiveWorldGenerator>,
    mut last_chunk: Local<Option<ChunkPos>>,
    mut pending_load: Local<VecDeque<ChunkPos>>,
    mut priority_queue: ResMut<PriorityMeshQueue>,
) {
    let Ok(player_transform) = player_query.single() else { return };
    let player_chunk = ChunkPos::from_world(player_transform.translation);

    if *last_chunk != Some(player_chunk) {
        *last_chunk = Some(player_chunk);

        // Unload far chunks
        world.chunks.retain(|pos, _| {
            (pos.0 - player_chunk.0).abs() <= LOAD_RADIUS
                && (pos.1 - player_chunk.1).abs() <= LOAD_RADIUS
                && (pos.2 - player_chunk.2).abs() <= LOAD_RADIUS
        });

        // Hard shell: generate synchronously so player is always blocked by solid terrain
        for dx in -HARD_RADIUS..=HARD_RADIUS {
            for dy in -HARD_RADIUS..=HARD_RADIUS {
                for dz in -HARD_RADIUS..=HARD_RADIUS {
                    let pos = ChunkPos(
                        player_chunk.0 + dx,
                        player_chunk.1 + dy,
                        player_chunk.2 + dz,
                    );
                    if !world.chunks.contains_key(&pos) {
                        world.chunks.insert(pos, generator.0.generate_chunk(pos));
                        priority_queue.0.push(pos);
                    }
                }
            }
        }

        // Soft queue: the rest load gradually, sorted closest-first
        pending_load.clear();
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
                        pending_load.push_back(pos);
                    }
                }
            }
        }
        pending_load.make_contiguous().sort_unstable_by_key(|p| {
            (p.0 - player_chunk.0).abs()
                + (p.1 - player_chunk.1).abs()
                + (p.2 - player_chunk.2).abs()
        });
    }

    // Gradual budget for soft queue
    let mut generated = 0;
    while generated < MAX_CHUNKS_PER_FRAME {
        let Some(pos) = pending_load.pop_front() else { break };
        if !world.chunks.contains_key(&pos) {
            world.chunks.insert(pos, generator.0.generate_chunk(pos));
            generated += 1;
        }
    }
}
