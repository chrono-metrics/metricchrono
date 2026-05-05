# Rust API

Primary crate:

```sh
cargo add metricchrono-core
```

Core imports:

```rust
use metricchrono_core::{geometric_ladder, ladder_distance, tick_distance, Absolute, Tier};
```

Checked constructors return `Result`. Checked distance APIs reject invalid
parameters, invalid ladder shape, undersized output buffers, and invalid
distances. Hot-path helpers write into caller-provided output buffers where
possible.

`metricchrono-log` re-exports the open in-memory event log, including
`EventId`, `append`, `next_event`, `iter_events`, `get`, `len`, and `is_empty`.
`metricchrono-consensus` re-exports the minimal consensus tick-field helpers,
including `SourceTick`, `ConsensusInput`, `weighted_consensus`,
`weighted_consensus_tierwise`, `tier_residuals`, `coherence_residual`, and
`simple_weight_update`. The implementations remain single-sourced in
`metricchrono-core` for v0.1.0.

See `crates/metricchrono-core/examples` for runnable examples.
