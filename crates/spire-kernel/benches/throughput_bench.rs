//! Covers:
//! - RAMBO event generation scaling with final-state multiplicity (N=2..5)
//! - Dirac trace evaluation hot path
//! - 10k-event Monte Carlo integration baseline

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use spire_kernel::algebra::{evaluate_cas_trace, CasExpr, LorentzIndex, SpacetimeDimension};
use spire_kernel::integration::{compute_cross_section, UniformIntegrator};
use spire_kernel::kinematics::{PhaseSpaceGenerator, RamboGenerator};

fn make_gamma_trace_expr() -> CasExpr {
    CasExpr::Mul(vec![
        CasExpr::GammaMat {
            index: LorentzIndex::Named("mu".to_string()),
        },
        CasExpr::GammaMat {
            index: LorentzIndex::Named("nu".to_string()),
        },
        CasExpr::GammaMat {
            index: LorentzIndex::Named("rho".to_string()),
        },
        CasExpr::GammaMat {
            index: LorentzIndex::Named("sigma".to_string()),
        },
    ])
}

fn bench_rambo_multiplicity(c: &mut Criterion) {
    let mut group = c.benchmark_group("rambo_generation");

    for n_final in [2_usize, 3, 4, 5] {
        let masses = vec![0.10566; n_final];
        let mut generator = RamboGenerator::with_seed(1234 + n_final as u64);

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("generate_event", n_final),
            &masses,
            |b, masses| {
                b.iter(|| black_box(generator.generate_event(200.0, masses).unwrap()));
            },
        );
    }

    group.finish();
}

fn bench_cas_trace(c: &mut Criterion) {
    let expr = make_gamma_trace_expr();
    let dim = SpacetimeDimension::four();

    c.bench_function("cas_trace_gamma4", |b| {
        b.iter(|| black_box(evaluate_cas_trace(&expr, dim).unwrap()))
    });
}

fn bench_cross_section_10k(c: &mut Criterion) {
    let masses = vec![0.10566, 0.10566];
    let integrator = UniformIntegrator::new();

    c.bench_function("cross_section_10k_events", |b| {
        b.iter(|| {
            let mut generator = RamboGenerator::with_seed(54321);
            let result = compute_cross_section(
                |_ps| 1.0,
                &integrator,
                &mut generator,
                200.0,
                &masses,
                10_000,
            )
            .unwrap();
            black_box(result)
        })
    });
}

fn bench_integrate_parallel_10k(c: &mut Criterion) {
    let masses = vec![0.10566, 0.10566];
    let integrator = UniformIntegrator::new();

    c.bench_function("integrate_parallel_10k_events", |b| {
        b.iter(|| {
            let mut generator = RamboGenerator::with_seed(24680);
            let result = integrator
                .integrate_parallel(
                    |ps| {
                        let p0 = &ps.momenta[0];
                        let p1 = &ps.momenta[1];
                        let spatial = p0[1] * p0[1] + p0[2] * p0[2] + p1[1] * p1[1] + p1[2] * p1[2];
                        1.0 + 1e-6 * spatial
                    },
                    &mut generator,
                    200.0,
                    &masses,
                    10_000,
                )
                .unwrap();
            black_box(result)
        })
    });
}

criterion_group!(
    throughput_benches,
    bench_rambo_multiplicity,
    bench_cas_trace,
    bench_cross_section_10k,
    bench_integrate_parallel_10k,
);
criterion_main!(throughput_benches);
