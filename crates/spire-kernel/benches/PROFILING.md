# SPIRE Kernel Throughput Profiling

This document tracks benchmark coverage and recent throughput measurements for `spire-kernel` hot paths.

## Benchmark Suites

- `kinematics_bench`:
  - `vector_add_10k`
  - `dot_product_10k`
  - `four_momentum_dot_10k`
  - `rambo_4body_1k_events`
  - `cross_section_1k_events`
  - `vector_scale_10k`
- `throughput_bench`:
  - `rambo_generate_event_{2,3,4,5}`
  - `cas_trace_gamma4`
  - `cross_section_10k_events`
  - `integrate_parallel_10k_events`

## Commands

From repository root:

- `cargo check -p spire-kernel --benches`
- `cargo bench -p spire-kernel --bench throughput_bench -- --sample-size 10`
- `cargo bench -p spire-kernel --bench throughput_bench integrate_parallel_10k_events -- --sample-size 20`

Optional chunk-size override for sensitivity checks:

- `SPIRE_RAYON_CHUNK_SIZE=1 cargo bench -p spire-kernel --bench throughput_bench integrate_parallel_10k_events -- --sample-size 20`

## Parallel Tuning Knobs

- `SPIRE_RAYON_CHUNK_SIZE`: overrides integration chunk size in:
  - `integration::UniformIntegrator::integrate_parallel`
  - `integration::UniformIntegrator::integrate_with_cuts_parallel`
- `SPIRE_ANALYSIS_CHUNK_SIZE`: overrides analysis chunk size in:
  - `analysis::run_analysis_parallel`

If unset, the kernel uses bounded adaptive chunking derived from event count.

## Latest Measurements (Windows, release bench run)

Measured with Criterion in this repository session.

- `rambo_generate_event_2`: ~2.918–3.259 µs
- `rambo_generate_event_3`: ~3.423–3.778 µs
- `rambo_generate_event_4`: ~4.135–4.538 µs
- `rambo_generate_event_5`: ~4.657–5.067 µs
- `cas_trace_gamma4`: ~12.869–14.286 µs
- `cross_section_10k_events`: ~27.163–29.886 ms
- `integrate_parallel_10k_events`: ~30.532–33.351 ms

Chunk override comparison (`integrate_parallel_10k_events`, sample size 20):

- Default adaptive chunking: ~29.857–32.549 ms
- Forced `SPIRE_RAYON_CHUNK_SIZE=1`: ~31.887–35.432 ms

Interpretation: tiny fixed chunks increase overhead and regress throughput relative to adaptive chunking.

## Notes

- Throughput runs should be compared on the same machine/profile due to Monte Carlo and scheduler variance.
- Keep kernel logic in `spire-kernel`; Tauri/frontend layers should only expose and consume results.