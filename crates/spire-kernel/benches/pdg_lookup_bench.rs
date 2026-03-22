//! PDG lookup benchmarks and latency gates for Phase 77.

use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spire_kernel::theory::pdg::adapter::PdgAdapter;
use spire_kernel::theory::pdg::contracts::PdgProvenance;

const MAX_CACHED_LOOKUP_US: f64 = 10.0;
const MAX_COLD_LOOKUP_US: f64 = 2_000.0;

fn resolve_pdg_db_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("SPIRE_PDG_DB_PATH") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../../data/pdg/pdg-2025-v0.2.2.sqlite"),
        manifest_dir.join("../../data/pdg/pdgall-2025-v0.2.2.sqlite"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn provenance_for(path: &Path) -> PdgProvenance {
    let path_str = path.to_string_lossy().to_string();
    PdgProvenance {
        edition: "PDG 2025".to_string(),
        release_timestamp: Some("2025-03-20T00:00:00Z".to_string()),
        source_id: "benchmark_local_sqlite".to_string(),
        origin: Some(path_str.clone()),
        source_path: Some(path_str),
        extraction_policy: Some("Catalog".to_string()),
        source_arbitration_outcome: Some("local".to_string()),
        local_file_fingerprint: None,
        fingerprint: "bench-v0.2.2".to_string(),
    }
}

fn open_adapter(path: &Path) -> PdgAdapter {
    PdgAdapter::open(path, provenance_for(path))
        .unwrap_or_else(|err| panic!("failed to open PDG adapter for benchmark: {err}"))
}

fn cached_lookup_latency_us(adapter: &PdgAdapter, mcid: i32, iterations: usize) -> f64 {
    let _ = adapter.lookup_particle_by_mcid(mcid);
    let started = Instant::now();
    for _ in 0..iterations {
        let record = adapter
            .lookup_particle_by_mcid(mcid)
            .unwrap_or_else(|err| panic!("cached lookup failed: {err}"));
        black_box(record);
    }
    started.elapsed().as_secs_f64() * 1_000_000.0 / iterations as f64
}

fn cold_lookup_latency_us(adapter: &PdgAdapter, mcid: i32, iterations: usize) -> f64 {
    let started = Instant::now();
    for _ in 0..iterations {
        adapter.clear_cache_entries();
        let record = adapter
            .lookup_particle_by_mcid(mcid)
            .unwrap_or_else(|err| panic!("cold lookup failed: {err}"));
        black_box(record);
    }
    started.elapsed().as_secs_f64() * 1_000_000.0 / iterations as f64
}

fn enforce_latency_gates(path: &Path) {
    let adapter = open_adapter(path);
    let mcid = 11;

    let cached_us = cached_lookup_latency_us(&adapter, mcid, 2_000);
    let cold_us = cold_lookup_latency_us(&adapter, mcid, 300);

    assert!(
        cached_us <= MAX_CACHED_LOOKUP_US,
        "cached lookup gate failed: {:.3}µs > {:.3}µs",
        cached_us,
        MAX_CACHED_LOOKUP_US
    );
    assert!(
        cold_us <= MAX_COLD_LOOKUP_US,
        "cold lookup gate failed: {:.3}µs > {:.3}µs",
        cold_us,
        MAX_COLD_LOOKUP_US
    );
}

fn bench_pdg_lookup(c: &mut Criterion) {
    let Some(path) = resolve_pdg_db_path() else {
        eprintln!("Skipping PDG lookup benchmark: no PDG sqlite database found.");
        return;
    };

    enforce_latency_gates(&path);

    let adapter = open_adapter(&path);

    c.bench_function("pdg_lookup_cached_electron", |b| {
        b.iter(|| {
            let record = adapter
                .lookup_particle_by_mcid(11)
                .unwrap_or_else(|err| panic!("cached benchmark lookup failed: {err}"));
            black_box(record)
        })
    });

    c.bench_function("pdg_lookup_cold_electron", |b| {
        b.iter(|| {
            adapter.clear_cache_entries();
            let record = adapter
                .lookup_particle_by_mcid(11)
                .unwrap_or_else(|err| panic!("cold benchmark lookup failed: {err}"));
            black_box(record)
        })
    });
}

criterion_group!(benches, bench_pdg_lookup);
criterion_main!(benches);
