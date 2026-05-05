# MetricChrono

MetricChrono is a deterministic Rust implementation of the epsilon-delta-p tick
primitive and its multiscale ladder representation.

It turns raw distances between states, events, embeddings, or signals into a
compact multiscale tick code:

- small changes stay silent below an explicit epsilon threshold;
- meaningful changes produce stable integer-like evidence at one or more
  resolutions;
- the same ladder can be evaluated from Rust, C, Python, and JavaScript;
- event logs can skip directly between salient changes without losing the full
  stream.

Use MetricChrono when wall-clock time is the wrong feature and you need a
deterministic answer to: "what changed, and at what scale?" Typical inputs are
vector embeddings, telemetry windows, simulation states, robot observations,
log-derived features, or any domain object with a distance metric.

This repository contains:

- `metricchrono-core`: the canonical Rust kernel, ladder utilities, metric
  traits, smooth surrogate, basic in-memory event log, adaptive zoom helpers,
  and minimal consensus tick field.
- `metricchrono-ffi`: a C ABI over the allocation-free hot paths and the basic
  event log.
- `bindings/python`: a thin `ctypes` wrapper over the shared library.
- `bindings/js`: a zero-dependency JS reference wrapper with a WASM-loader hook.

Product-specific deployment tooling, hosted services, and organization-specific
integrations are out of scope for this repository.

## Why MetricChrono

Clock time says when a sample arrived. MetricChrono describes how much a system
has moved through a metric space. That distinction matters when two streams have
the same sampling rate but very different behavior, or when a single stream has
quiet periods followed by sharp state changes.

The core tick is intentionally small:

```text
T(d) = (epsilon / epsilon_ref)^p * ceil(d / delta) * 1[d >= epsilon]
```

A ladder runs that comparator at multiple scales, producing a fixed-width code
that is easy to store, compare, index, and feed into downstream models.

## Rust Quick Start

```rust
use metricchrono_core::{
    geometric_ladder, ladder_pair, ladder_values, tick_distance, Euclidean, MetricChronoError,
    Tier,
};

fn main() -> Result<(), MetricChronoError> {
    let tier = Tier::new(1.0, 0.5, 0.5, 1.0)?;
    assert_eq!(tick_distance(0.25, tier), 0.0);

    let ladder = geometric_ladder(0.5, 0.5, 2.0, 4, 0.5, 1.0)?;
    let ticks = ladder_values(3.0, &ladder)?;
    println!("{ticks:?}");

    let metric = Euclidean;
    let paired = ladder_pair(&[0.0, 0.0][..], &[3.0, 4.0][..], &metric, &ladder)?;
    println!("{paired:?}");

    Ok(())
}
```

## Build And Test

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p metricchrono-core --example basic
cargo bench -p metricchrono-core --bench ladder_throughput
cargo bench -p metricchrono-core --bench clock_only_comparison
(cd bindings/js && npm test)
python3 -m pip wheel bindings/python --no-deps -w /tmp/metricchrono-wheel
```

Build the C ABI shared library:

```sh
cargo build -p metricchrono-ffi --release
```

The Python wheel builds and bundles the native FFI library when Cargo is
available. For direct source-tree use without installing the wheel, point the
wrapper at a locally built library:

```sh
export METRICCHRONO_FFI_LIB=target/release/libmetricchrono_ffi.dylib
python -c "import metricchrono; print(metricchrono.tick_distance(1.2, metricchrono.Tier(0.5, 1.0, 0.5, 1.0)))"
```

On Linux the shared library suffix is `.so`; on Windows it is `.dll`.

## Release Readiness

This repository is designed to publish in ordered layers: Rust core first, then
the Rust FFI crate, then language wrappers. The full checklist is in
[docs/release.md](docs/release.md). In particular, `metricchrono-ffi` cannot be
published until `metricchrono-core` is already visible in the crates.io index.

## Public API Surface

The public core is deliberately small:

- `tick_distance(d, tier)` computes a single epsilon-delta-p tick.
- `ladder_distance(d, ladder, out)` computes a deterministic multiscale vector.
- `Metric<T>` lets callers plug in Euclidean, squared Euclidean, Manhattan,
  cosine, KL-like, Jensen-Shannon, diagonal Mahalanobis, or custom distances.
- `smooth_tick_distance` and `smooth_ladder_distance` provide differentiable
  surrogates for ML and RL experiments.
- `EventLog` is a basic in-memory event skip-list for salient tier jumps.
- `PromotionCounter`, `carry_rules`, and `normalize_ticks` cover basic ladder
  carry and normalization helpers.
- `adaptive_ladder_distance` and `adaptive_zoom_window` expose early-stop and
  zoom helpers for edge and embedded use.
- `weighted_consensus`, `coherence_residuals`, and `simple_weight_update`
  provide a minimal consensus tick field.

See [docs/scope.md](docs/scope.md) for the repository scope.
