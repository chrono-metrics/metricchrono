# Rust API

Primary crate:

```sh
cargo add metricchrono-core
```

Core imports:

```rust
use metricchrono_core::{geometric_ladder, ladder_distance, tick_distance, Tier};
```

Checked constructors return `Result`. Checked distance APIs reject invalid
parameters, invalid ladder shape, undersized output buffers, and invalid
distances. Hot-path helpers write into caller-provided output buffers where
possible.

See `crates/metricchrono-core/examples` for runnable examples.
