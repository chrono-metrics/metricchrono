# Changelog

## Unreleased

- Added: `GuardedQuantileThreshold`, `GuardedAmbientScale`, and
  `CfarDecision` in `metricchrono-core` for causal guarded ambient CFAR
  thresholds, detection floors, and robust distance normalization.

## 0.4.0 (2026-06-11)

- Added: `CoverageMeter`, `progress_efficiency`, `classify_regime`, and
  `OperatingRegime` in `metricchrono-core` — a per-tier streaming coverage
  read-out (greedy maximal epsilon-packing of the visited image, holding
  representatives only) complementary to the per-step tick throughput.
  Coverage is invariant under revisits and detects sub-threshold relocation
  (creep) that per-step thresholding is silent on by design; jointly with
  throughput it classifies windows into quiescent / progress / churn / creep.
- Performance: `CoverageMeter` stores pooled representatives (a sample
  admitted at several tiers is cloned once and indexed per tier), memoizes
  each representative's distance across tiers within one observation, scans
  newest-first so locality-heavy streams short-circuit early (2.3x on the
  2-D walk benchmark, `benches/coverage_throughput.rs`), and exposes the
  allocation-free `observe_into`. `representatives()` now returns an
  iterator over pooled entries and accessors no longer require `T: Clone`.
- Added: full binding parity for coverage. C ABI: opaque `MCCoverageMeter`
  (`mc_coverage_meter_new/observe/counts/unique_representatives/tier_count/
  free`) over fixed-dimension `double` states with the existing metric ids,
  plus `mc_progress_efficiency`, `mc_classify_regime`, and the `MCRegime`
  enum, declared in both copies of `metricchrono.h` and exercised by the C
  header smoke test. Python: `CoverageMeter`, `OperatingRegime`,
  `classify_regime`, `progress_efficiency` over the C ABI. JavaScript:
  pure-JS `CoverageMeter` (pooled storage, per-observe distance memo,
  newest-first scan, NaN-rejects semantics identical to Rust),
  `OperatingRegime`, `classifyRegime`, `progressEfficiency`, with TypeScript
  declarations and tests.
- Added: custom distance callbacks for the C-ABI coverage meter.
  `mc_coverage_meter_new_with_callback` takes an `MCDistanceFn`
  (`double (*)(const double *a, const double *b, size_t dim, void *user_data)`),
  so embedders can audit coverage under domain metrics without round-tripping
  states; returning NaN rejects admission (the safe failure mode). The Python
  `CoverageMeter` accepts any Python callable as its metric (exceptions in the
  callable are converted to NaN rather than unwinding into the C ABI); the
  JavaScript binding already accepted arbitrary metric functions.

## 0.3.0 (2026-06-09)

- Added: Pre-built binary wheels on PyPI for Linux (manylinux x86_64 and
  aarch64), macOS (Intel and Apple Silicon), and Windows (x86_64), so
  `pip install metricchrono` no longer requires a Rust toolchain on those
  platforms. The source distribution still builds from source (and so still
  requires Cargo). No API changes.

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
