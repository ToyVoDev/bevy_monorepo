use bevy::prelude::*;
use std::collections::HashMap;
use crate::chunk::Chunk;
use crate::types::ChunkPos;
use crate::world::ActiveWorldGenerator;

pub const LOAD_RADIUS: i32 = 10;

#[derive(Resource, Default, Debug)]
pub struct ChunkedWorld {
    pub(crate) chunks: HashMap<ChunkPos, Chunk>,
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
) {
    let Ok(player_transform) = player_query.single() else { return };
    let player_chunk = ChunkPos::from_world(player_transform.translation);

    if *last_chunk == Some(player_chunk) {
        return;
    }
    *last_chunk = Some(player_chunk);

    let diameter = (2 * LOAD_RADIUS + 1) as usize;
    let mut desired = std::collections::HashSet::with_capacity(diameter * diameter * diameter);
    let r = LOAD_RADIUS;
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                desired.insert(ChunkPos(
                    player_chunk.0 + dx,
                    player_chunk.1 + dy,
                    player_chunk.2 + dz,
                ));
            }
        }
    }

    for &pos in &desired {
        world.chunks.entry(pos).or_insert_with(|| generator.0.generate_chunk(pos));
    }

    world.chunks.retain(|pos, _| desired.contains(pos));
}
