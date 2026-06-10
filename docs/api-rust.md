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

`CoverageMeter` is the complementary history read-out to per-step ticks: a
per-tier streaming greedy epsilon-packing of the visited image (pooled
representatives, allocation-free `observe_into`). `progress_efficiency` and
`classify_regime` summarise windows on the quiescent / progress / churn /
creep quadrant; creep — relocation through individually sub-threshold steps —
is exactly what per-step thresholding cannot see, and coverage audits it with
bounded memory.

See `crates/metricchrono-core/examples` for runnable examples.
