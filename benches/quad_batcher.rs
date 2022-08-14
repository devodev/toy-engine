use cgmath::{Vector3, Vector4};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use toy_engine::QuadBatcher;

const DEFAULT_MAX_QUADS: u32 = 2000;

fn bench_quad_batcher_add_quad(c: &mut Criterion) {
    let mut quad_batcher = QuadBatcher::new(DEFAULT_MAX_QUADS);
    c.bench_function("QuadBatcher::add_quad", |b| {
        b.iter(|| {
            quad_batcher.add_quad(
                black_box(Vector3::new(0.0, 0.0, 0.0)),
                black_box(Vector3::new(0.0, 0.0, 0.0)),
                black_box(Vector4::new(0.0, 0.0, 0.0, 0.0)),
            )
        })
    });
}

fn bench_quad_batcher_add_quad_10(c: &mut Criterion) {
    let mut quad_batcher = QuadBatcher::new(DEFAULT_MAX_QUADS);
    c.bench_function("QuadBatcher::add_quad 10", |b| {
        b.iter(|| {
            for _ in 0..10 {
                quad_batcher.add_quad(
                    black_box(Vector3::new(0.0, 0.0, 0.0)),
                    black_box(Vector3::new(0.0, 0.0, 0.0)),
                    black_box(Vector4::new(0.0, 0.0, 0.0, 0.0)),
                )
            }
        })
    });
}

fn bench_quad_batcher_add_quad_100(c: &mut Criterion) {
    let mut quad_batcher = QuadBatcher::new(DEFAULT_MAX_QUADS);
    c.bench_function("QuadBatcher::add_quad 100", |b| {
        b.iter(|| {
            for _ in 0..100 {
                quad_batcher.add_quad(
                    black_box(Vector3::new(0.0, 0.0, 0.0)),
                    black_box(Vector3::new(0.0, 0.0, 0.0)),
                    black_box(Vector4::new(0.0, 0.0, 0.0, 0.0)),
                )
            }
        })
    });
}

fn bench_quad_batcher_add_quad_1000(c: &mut Criterion) {
    let mut quad_batcher = QuadBatcher::new(DEFAULT_MAX_QUADS);
    c.bench_function("QuadBatcher::add_quad 1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                quad_batcher.add_quad(
                    black_box(Vector3::new(0.0, 0.0, 0.0)),
                    black_box(Vector3::new(0.0, 0.0, 0.0)),
                    black_box(Vector4::new(0.0, 0.0, 0.0, 0.0)),
                )
            }
        })
    });
}

fn bench_quad_batcher_add_quad_10000(c: &mut Criterion) {
    let mut quad_batcher = QuadBatcher::new(DEFAULT_MAX_QUADS);
    c.bench_function("QuadBatcher::add_quad 10000", |b| {
        b.iter(|| {
            for _ in 0..10000 {
                quad_batcher.add_quad(
                    black_box(Vector3::new(0.0, 0.0, 0.0)),
                    black_box(Vector3::new(0.0, 0.0, 0.0)),
                    black_box(Vector4::new(0.0, 0.0, 0.0, 0.0)),
                )
            }
        })
    });
}

fn bench_quad_batcher_add_quad_grid(c: &mut Criterion) {
    let mut quad_batcher = QuadBatcher::new(DEFAULT_MAX_QUADS);
    c.bench_function("QuadBatcher::add_quad grid", |b| {
        b.iter(|| {
            for x in 0..51 {
                for y in 0..51 {
                    let color = Vector4::new(x as f32 / 50.0, y as f32 / 50.0, 0.7, 1.0);
                    let x = x as f32 - 25.0;
                    let y = y as f32 - 25.0;
                    quad_batcher.add_quad(
                        black_box(Vector3::new(x, y, 1.0)),
                        black_box(Vector3::new(0.02, 0.02, 1.0)),
                        black_box(color),
                    );
                }
            }
        })
    });
}

criterion_group!(
    benches,
    bench_quad_batcher_add_quad,
    bench_quad_batcher_add_quad_10,
    bench_quad_batcher_add_quad_100,
    bench_quad_batcher_add_quad_1000,
    bench_quad_batcher_add_quad_10000,
    bench_quad_batcher_add_quad_grid,
);
criterion_main!(benches);
