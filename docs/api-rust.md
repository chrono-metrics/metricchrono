# Rust API

Primary crate:

```sh
cargo add metricchrono-core
```

Core imports:

```rust
use metricchrono_core::{geometric_ladder, ladder_distance, tick_distance, Absolute, Tier};
```

Default builds expose `Absolute`, `Euclidean`, `MetricFn`, the `Metric` trait,
and `tick_pair`/`ladder_pair`. Use `features = ["metrics-extra"]` for
`SquaredEuclidean`, `Manhattan`, `Cosine`, `KullbackLeibler`, `JensenShannon`,
and `DiagonalMahalanobis`.

Checked constructors return `Result`. Checked distance APIs reject invalid
parameters, invalid ladder shape, undersized output buffers, and invalid
distances. Hot-path helpers write into caller-provided output buffers where
possible.

The v0.1 `metricchrono-log` and `metricchrono-consensus` re-export crates were
removed in v0.2. Depend on `metricchrono-core` and import `EventLog`,
`EventRecord`, `EventSummary`, `weighted_consensus`, `coherence_residual`, and
related APIs directly from `metricchrono_core`.

See `crates/metricchrono-core/examples` for runnable examples.
