use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use voxel_game::chunk::Chunk;
use voxel_game::chunk::loading::ChunkedWorld;
use voxel_game::simulation::debris::{DebrisParticle, tick_debris, solidify_resting_debris};
use voxel_game::types::{ChunkPos, LocalVoxelPos, STONE};
use voxel_game::config::CHUNK_SIZE;
use std::time::Duration;

#[test]
fn debris_falls_and_stops() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Advance time by a fixed 1/60s per update
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(1.0 / 60.0)));
    app.init_resource::<ChunkedWorld>();

    {
        let mut world = app.world_mut().resource_mut::<ChunkedWorld>();
        let mut floor_chunk = Chunk::new();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                floor_chunk.set(LocalVoxelPos::new(x as u8, 0, z as u8), STONE);
            }
        }
        world.chunks.insert(ChunkPos(0, 0, 0), floor_chunk);
    }

    let debris_id = app.world_mut().spawn((
        DebrisParticle::new(STONE, Vec3::ZERO),
        Transform::from_xyz(0.5, 2.0, 0.5),
    )).id();

    app.add_systems(Update, tick_debris);

    for _ in 0..120 {
        app.update();
    }

    let transform = app.world().entity(debris_id).get::<Transform>().unwrap();
    assert!(transform.translation.y < 0.5, "debris should have fallen: y={}", transform.translation.y);
    assert!(transform.translation.y > 0.0, "debris should rest above floor, not clip through: y={}", transform.translation.y);
}

#[test]
fn debris_solidifies_into_chunk() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(1.0 / 60.0)));
    app.init_resource::<ChunkedWorld>();

    {
        let mut world = app.world_mut().resource_mut::<ChunkedWorld>();
        let mut chunk = Chunk::new();
        // Place a solid floor at y=4 so the debris at y=5 rests on it
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                chunk.set(LocalVoxelPos::new(x as u8, 4, z as u8), STONE);
            }
        }
        world.chunks.insert(ChunkPos(0, 0, 0), chunk);
    }

    // Debris sitting at voxel (5, 5, 5) — just above the y=4 floor layer.
    // Start with rest_timer near the solidify threshold so it tips over quickly.
    app.world_mut().spawn((
        DebrisParticle {
            voxel_id: STONE,
            velocity: Vec3::ZERO,
            rest_timer: 4.9,
        },
        Transform::from_xyz(0.55, 0.55, 0.55),
    ));

    app.add_systems(Update, (tick_debris, solidify_resting_debris));

    // Run enough updates: ~7 frames at 1/60s tips rest_timer over 5.0
    for _ in 0..15 {
        app.update();
    }

    let world = app.world().resource::<ChunkedWorld>();
    let chunk = world.get(ChunkPos(0, 0, 0)).expect("chunk should exist");
    let local = LocalVoxelPos::new(5, 5, 5);
    assert_eq!(chunk.get(local), STONE, "debris should have solidified into chunk");
}
