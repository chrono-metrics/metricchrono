# metricchrono-core

Canonical Rust implementation of the public MetricChrono primitive:
single-scale epsilon-delta-p ticks, multiscale ladders, stock metric examples,
smooth surrogates, basic event memory, adaptive zoom, and minimal consensus.

## Quick Start

```rust
use metricchrono_core::{geometric_ladder, ladder_values, tick_distance, Tier};

fn main() -> metricchrono_core::Result<()> {
    let tier = Tier::new(0.5, 1.0, 0.5, 1.0)?;
    assert_eq!(tick_distance(0.25, tier), 0.0);

    let ladder = geometric_ladder(0.5, 1.0, 2.0, 4, 0.5, 1.0)?;
    let ticks = ladder_values(3.0, &ladder)?;
    println!("{ticks:?}");

    Ok(())
}
```

## Public Modules

- `Tier`, `tick_distance`, and `ladder_distance` implement the deterministic
  epsilon-delta-p kernel.
- The `Metric<T>` trait, `Absolute`, `Euclidean`, `MetricFn`, and
  `tick_pair`/`ladder_pair` cover the default metric API.
  `SquaredEuclidean`, `Manhattan`, `Cosine`, `KullbackLeibler`,
  `JensenShannon`, and `DiagonalMahalanobis` are available with
  `features = ["metrics-extra"]`.
- `smooth_tick_distance` and `smooth_ladder_distance` expose differentiable
  surrogates for ML and RL experiments.
- `EventLog` is a basic in-memory event skip-list for tier-local salient
  changes.
- `adaptive_ladder_distance` and `adaptive_zoom_window` provide early-stop and
  zoom helpers.
- `weighted_consensus`, `coherence_residuals`, and `simple_weight_update`
  provide a minimal consensus tick field.

## Verification

```sh
cargo test -p metricchrono-core --all-targets
cargo bench -p metricchrono-core --bench clock_only_comparison
cargo bench -p metricchrono-core --bench ladder_throughput
```

See the repository README for broader examples and project scope.
