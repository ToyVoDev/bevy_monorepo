use std::sync::Arc;
use bevy::prelude::*;
use bevy::prelude::TaskPoolPlugin;
use voxel_game::chunk::loading::{
    ChunkedWorld, PendingGeneration, GeneratingChunks,
    spawn_generation_tasks, collect_generated_chunks,
};
use voxel_game::chunk::Chunk;
use voxel_game::types::ChunkPos;
use voxel_game::world::{ActiveWorldGenerator, WorldGenerator};

struct NullGenerator;
impl WorldGenerator for NullGenerator {
    fn generate_chunk(&self, _pos: ChunkPos) -> Chunk { Chunk::new() }
}

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());
    app.init_resource::<ChunkedWorld>();
    app.init_resource::<PendingGeneration>();
    app.init_resource::<GeneratingChunks>();
    app.insert_resource(ActiveWorldGenerator(Arc::new(NullGenerator)));
    app.add_systems(Update, (
        spawn_generation_tasks,
        collect_generated_chunks.after(spawn_generation_tasks),
    ));
    app
}

#[test]
fn all_queued_positions_are_generated() {
    let mut app = make_app();

    {
        let mut pending = app.world_mut().resource_mut::<PendingGeneration>();
        for x in 0..10i32 {
            pending.0.push_back(ChunkPos(x, 0, 0));
        }
    }

    for _ in 0..200 {
        app.update();
        let count = app.world().resource::<ChunkedWorld>().chunks.len();
        if count >= 10 { break; }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let world = app.world().resource::<ChunkedWorld>();
    assert_eq!(world.chunks.len(), 10, "all 10 queued chunks should be generated");
    for x in 0..10i32 {
        assert!(world.chunks.contains_key(&ChunkPos(x, 0, 0)),
            "chunk ({x},0,0) missing");
    }
}

#[test]
fn in_flight_edit_is_not_lost() {
    let mut app = make_app();

    {
        let mut pending = app.world_mut().resource_mut::<PendingGeneration>();
        pending.0.push_back(ChunkPos(0, 0, 0));
        pending.0.push_back(ChunkPos(0, 0, 0)); // duplicate
    }

    for _ in 0..200 {
        app.update();
        let count = app.world().resource::<ChunkedWorld>().chunks.len();
        if count >= 1 { break; }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let world = app.world().resource::<ChunkedWorld>();
    assert_eq!(world.chunks.len(), 1, "duplicate positions should not produce duplicate chunks");
}
