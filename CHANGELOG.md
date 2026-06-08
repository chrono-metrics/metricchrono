# Changelog

## 0.2.0 (2026-06-08)

- BREAKING: Removed the empty `metricchrono-consensus` and `metricchrono-log`
  re-export crates. Migration: depend on `metricchrono-core` and import those
  APIs directly.
- BREAKING: Moved `SquaredEuclidean`, `Manhattan`, `Cosine`,
  `KullbackLeibler`, `JensenShannon`, and `DiagonalMahalanobis` behind the
  default-off `metrics-extra` Cargo feature. These names do not resolve in a
  default build. Migration: use
  `metricchrono-core = { version = "0.2", features = ["metrics-extra"] }`.
- BREAKING: Removed the C ABI export `mc_tick_distance_unchecked`; the safe
  `mc_tick_distance` and the primitive, allocation-free
  `mc_tick_distance_raw` used by the WASM binding remain exported.
  Migration: call `mc_tick_distance` for safe use or `mc_tick_distance_raw` for
  the WASM hot path.
- BREAKING (JS/npm): The `@metricchrono/core` binding no longer exports the six
  extra distance functions (`cosineDistance`, `kullbackLeiblerDistance`,
  `jensenShannonDistance`, `manhattanDistance`, `squaredEuclideanDistance`,
  `diagonalMahalanobisDistance`), matching the Rust `metrics-extra` gating; the
  JS surface is now a thin projection of the C ABI keep-set. `*FromSchema` now
  rejects unknown fields, and `EventLog` rejects out-of-range index/tier.
- Added: The C ABI and Python binding now expose the full keep-set, so the
  EventLog skip-list is finally navigable from Python — `first_event`,
  `compact_summary`, a record reader, `tier_count`, `is_empty` — plus the default
  `Euclidean`/`Absolute` metrics with `tick_pair`/`ladder_pair`, `custom_ladder`,
  `validate_ladder`, `normalize_ticks`, `carry_rules`, a `PromotionCounter`
  handle, a structured error channel (`mc_last_error_message`), and a
  `METRICCHRONO_ABI_VERSION` macro. Python now validates schema input (rejecting
  invalid tiers and unknown fields) and surfaces structured error messages.
- Added: A cross-language binding-conformance suite — fixtures generated from the
  Rust core and asserted by both the Python and JS harnesses, including rejection
  cases — so the bindings cannot silently diverge from the core.
- Fixed: Two C ABI memory-safety issues surfaced by review. `mc_error_message`
  now takes an `int` (it previously took a Rust enum by value, which is undefined
  behavior if a C caller passes an out-of-range value); `mc_weighted_consensus`
  rejects element counts whose total byte length would overflow.

## 0.1.0

- Initial public Rust core crate.
- Single-scale epsilon-delta-p kernel and multiscale ladders.
- Basic metrics including Euclidean, Manhattan, cosine, KL-like,
  Jensen-Shannon, and diagonal Mahalanobis examples.
- Smooth differentiable surrogate.
- Basic in-memory event skip-list.
- Adaptive zoom and early-stop helpers.
- Minimal weighted consensus field.
- C ABI, Python ctypes wrapper, and JS/WASM wrapper path.
- Shared Rust/Python/JS golden fixtures.
