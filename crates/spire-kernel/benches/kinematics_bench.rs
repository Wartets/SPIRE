//! # Kinematics Benchmarks
//!
//! Criterion benchmarks measuring the nanosecond cost of core
//! operations on `SpacetimeVector`, `FourMomentum`, and the RAMBO
//! phase-space generator.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spire_kernel::algebra::{FourMomentum, MetricSignature, SpacetimeVector};
use spire_kernel::integration::{compute_cross_section, UniformIntegrator};
use spire_kernel::kinematics::{PhaseSpaceGenerator, RamboGenerator};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a batch of random 4D SpacetimeVectors using a seeded RNG.
fn make_vectors(n: usize, seed: u64) -> Vec<SpacetimeVector> {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    (0..n)
        .map(|_| {
            SpacetimeVector::new_4d(
                rng.gen_range(-100.0..100.0),
                rng.gen_range(-100.0..100.0),
                rng.gen_range(-100.0..100.0),
                rng.gen_range(-100.0..100.0),
            )
        })
        .collect()
}

/// Generate a batch of random FourMomentum values.
fn make_four_momenta(n: usize, seed: u64) -> Vec<FourMomentum> {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    (0..n)
        .map(|_| {
            FourMomentum::new(
                rng.gen_range(1.0..500.0),
                rng.gen_range(-200.0..200.0),
                rng.gen_range(-200.0..200.0),
                rng.gen_range(-200.0..200.0),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_vector_add(c: &mut Criterion) {
    let vecs_a = make_vectors(10_000, 42);
    let vecs_b = make_vectors(10_000, 99);

    c.bench_function("vector_add_10k", |b| {
        b.iter(|| {
            let mut sum = SpacetimeVector::zero(4);
            for (a, bv) in vecs_a.iter().zip(&vecs_b) {
                sum = black_box(sum + a.clone() + bv.clone());
            }
            sum
        })
    });
}

fn bench_dot_product(c: &mut Criterion) {
    let vecs_a = make_vectors(10_000, 42);
    let vecs_b = make_vectors(10_000, 99);
    let metric = MetricSignature::minkowski_4d();

    c.bench_function("dot_product_10k", |b| {
        b.iter(|| {
            let mut total = 0.0_f64;
            for (a, bv) in vecs_a.iter().zip(&vecs_b) {
                total += metric.inner_product(a, bv).unwrap();
            }
            black_box(total)
        })
    });
}

fn bench_four_momentum_dot(c: &mut Criterion) {
    let momenta_a = make_four_momenta(10_000, 42);
    let momenta_b = make_four_momenta(10_000, 99);

    c.bench_function("four_momentum_dot_10k", |b| {
        b.iter(|| {
            let mut total = 0.0_f64;
            for (a, bv) in momenta_a.iter().zip(&momenta_b) {
                total += a.dot(bv);
            }
            black_box(total)
        })
    });
}

fn bench_rambo_generation(c: &mut Criterion) {
    // e+e- → 4μ (massless approximation for speed)
    let masses = vec![0.10566, 0.10566, 0.10566, 0.10566];
    let cms_energy = 200.0;

    c.bench_function("rambo_4body_1k_events", |b| {
        b.iter(|| {
            let mut gen = RamboGenerator::with_seed(12345);
            for _ in 0..1_000 {
                let _event = black_box(gen.generate_event(cms_energy, &masses).unwrap());
            }
        })
    });
}

fn bench_cross_section(c: &mut Criterion) {
    // Constant |M|² = 1.0 for benchmarking overhead.
    let masses = vec![0.10566, 0.10566];
    let cms_energy = 200.0;
    let integrator = UniformIntegrator::new();

    c.bench_function("cross_section_1k_events", |b| {
        b.iter(|| {
            let mut gen = RamboGenerator::with_seed(54321);
            let result = compute_cross_section(
                |_ps| 1.0,
                &integrator,
                &mut gen,
                cms_energy,
                &masses,
                1_000,
            )
            .unwrap();
            black_box(result)
        })
    });
}

fn bench_spacetime_vector_scale(c: &mut Criterion) {
    let vecs = make_vectors(10_000, 42);

    c.bench_function("vector_scale_10k", |b| {
        b.iter(|| {
            for v in &vecs {
                black_box(v.scale(2.5));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_vector_add,
    bench_dot_product,
    bench_four_momentum_dot,
    bench_rambo_generation,
    bench_cross_section,
    bench_spacetime_vector_scale,
);
criterion_main!(benches);
