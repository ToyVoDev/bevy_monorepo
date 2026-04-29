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
