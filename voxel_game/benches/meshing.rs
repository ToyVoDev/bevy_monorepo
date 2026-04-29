use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use voxel_game::types::STONE;

fn bench_greedy_mesh(c: &mut Criterion) {
    let mut group = c.benchmark_group("greedy_mesh");

    let n = voxel_game::config::CHUNK_SIZE;

    for fill in [0.1f32, 0.5, 1.0] {
        let voxels: Vec<u16> = (0..n * n * n)
            .map(|i| {
                let x = i % n;
                let y = (i / n) % n;
                let z = i / (n * n);
                if (x as f32 / n as f32) < fill && (y + z) % 3 != 0 {
                    STONE
                } else {
                    0
                }
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new(format!("CHUNK_SIZE={n}"), format!("fill={fill:.1}")),
            &voxels,
            |b, v| b.iter(|| voxel_game::chunk::meshing::greedy_mesh(black_box(v))),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_greedy_mesh);
criterion_main!(benches);
