# PDG Data Semantics and Runtime Behavior

## Scope

This document defines the internal semantics used by SPIRE for Particle Data Group (PDG) ingestion, query resolution, and frontend behavior under large catalog loads.

The PDG integration is designed to be:

- deterministic across local and optional remote sources,
- reproducible across sessions via edition and provenance locks,
- explicit about generic vs physically samplable decay channels.

## Dataset Layout (`data/pdg/*`)

SPIRE treats `data/pdg/` as the canonical local dataset bundle.

Expected artifacts:

- `pdg-2025-v0.2.2.sqlite` (primary local SQLite source)
- `pdgall-*.sqlite` (expanded local catalogs where present)
- `mass_width_2025.txt` (text fallback for core mass/width seeding)
- optional auxiliary mapping/index artifacts

### Source precedence

Runtime precedence is:

1. local SQLite bundle (`pdg-*.sqlite`),
2. local fallback text snapshots for critical core quantities,
3. optional REST enrichment (strictly throttled, never mandatory).

The frontend remains fully usable in offline local-only mode.

## SQLite query semantics

## Why `pdgid.id` + `pdgid.data_type` is authoritative

SPIRE resolves quantities through the PDG identifier graph (`pdgid`) and **not** through `pdgdata.value_type`.

- `pdgid.id` is the stable relational join key linking quantity descriptors to value rows.
- `pdgid.data_type` carries physical quantity class (`M`, `G`, `T`, `BR`, ...).
- `pdgdata.value_type` is interpreted as formatting/limit metadata and must not be used to infer quantity class.

This prevents cross-quantity ambiguity and ensures that mass, width, lifetime, and branching rows are selected by explicit physical typing.

### Quantity extraction behavior

For each resolved root particle:

- Mass: `data_type = 'M'`
- Width: `data_type = 'G'`
- Lifetime: `data_type = 'T'`
- Branching rows: `data_type = 'BR'`

Value normalization is unit-aware:

- energy-like values are normalized to GeV,
- lifetime values are normalized to seconds,
- percentage display rows are converted to fractions.

## Uncertainty and limit semantics

SPIRE maps PDG values to one of:

- `Exact { value, is_limit }`
- `Symmetric { value, error, is_limit }`
- `Asymmetric { value, error: { minus, plus }, is_limit }`

`is_limit` is set when PDG rows indicate upper/lower/limit-style reporting (via limit metadata fields).

## Alias resolution and ambiguity handling

Particle resolution uses deterministic MCID/name/alias traversal through `pdgparticle`, `pdgitem`, and `pdgitem_map`.

For decays, product resolution distinguishes:

- `Concrete { mcid }` for explicitly resolvable products,
- `Generic { description }` for unresolved/inclusive placeholders (e.g. `X`, hadronic inclusive channels, or ambiguous aliases).

No unresolved product is silently dropped.

## Extraction policies

SPIRE supports two decay extraction policies:

- `StrictPhysical`
  - excludes channels containing generic products,
  - intended for physically samplable pipelines and strict event generation.

- `Catalog`
  - includes all reported channels, including generic/inclusive branches,
  - intended for reference browsing, completeness checks, and literature comparison.

## Edition lock and provenance

Every PDG-backed model update carries provenance fields:

- edition,
- source id/path,
- extraction policy,
- arbitration outcome,
- local file fingerprint.

Edition mismatch behavior is policy-controlled:

- `Strict`: reject mixed-edition injection (`EditionMismatch`),
- `AllowOverride`: allow explicit lock replacement.

This guarantees deterministic, auditable model state across sessions.

## Caching, streaming, and diagnostics

PDG runtime includes bounded caches for:

- particle records,
- decay tables,
- identifier resolution.

Diagnostics expose:

- cache hit/miss/eviction metrics,
- DB query counts and latency,
- progressive catalog load status,
- network queue/retry/429/503 telemetry (when live API is enabled).

Catalog mode uses chunked progressive fetch with cancellation support and non-progressive fallback for resilience.

## Optional REST enrichment

REST integration is optional and conservative:

- local data remains authoritative unless arbitration switches to API preference,
- request flow is token-bucket throttled,
- queue depth is bounded,
- retry/cooldown handles transient API pressure (e.g. 429/503),
- offline fallback is explicit in frontend status surfaces.

## Frontend high-volume behavior

In full-catalog browsing mode the UI provides:

- progressive record loading with request IDs,
- responsive search/sort/facet filtering,
- edition filtering with history compare,
- live/offline source badges and fallback notices,
- telemetry panel visibility into cache and network pressure.

These controls keep interaction stable under large PDG datasets without violating stateless IPC boundaries.
