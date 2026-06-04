# Changelog

## 0.2.0 (unreleased)

- BREAKING: Removed the empty `metricchrono-consensus` and `metricchrono-log`
  re-export crates. Migration: depend on `metricchrono-core` and import those
  APIs directly.
- BREAKING: Moved `SquaredEuclidean`, `Manhattan`, `Cosine`,
  `KullbackLeibler`, `JensenShannon`, and `DiagonalMahalanobis` behind the
  default-off `metrics-extra` Cargo feature. Migration: enable
  `features = ["metrics-extra"]`.
- BREAKING: Removed the C ABI export `mc_tick_distance_unchecked`; the safe
  `mc_tick_distance` and WASM hot-path `mc_tick_distance_raw` remain exported.
  Migration: call `mc_tick_distance` for safe use or `mc_tick_distance_raw` for
  the WASM hot path.

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
