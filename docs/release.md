# Release Checklist

This checklist keeps publication mechanical and ordered. It is intentionally
separate from the implementation scope in `docs/scope.md`.

## Repository

Before making the repository public:

```sh
git status --short --branch
git diff --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
cargo build -p metricchrono-ffi --release
cmp -s include/metricchrono.h crates/metricchrono-ffi/include/metricchrono.h
```

The GitHub repository should have a pushed `main` branch and `main` set as its
default branch before it is made public.

## Rust Crates

Publish in dependency order:

```sh
cargo publish --dry-run -p metricchrono-core
cargo publish -p metricchrono-core
```

Wait for `metricchrono-core` to appear in the crates.io index, then publish the
FFI crate:

```sh
cargo publish --dry-run -p metricchrono-ffi
cargo publish -p metricchrono-ffi
```

The FFI dry-run is expected to fail before the core crate exists on crates.io,
because `metricchrono-ffi` depends on `metricchrono-core = "0.1.0"`.

## JavaScript

```sh
npm test --prefix bindings/js
npm pack --dry-run --prefix bindings/js
```

The package currently publishes as `@metricchrono/core`.

## Python

Build a wheel from the repository root:

```sh
python3 -m pip wheel bindings/python --no-deps -w /tmp/metricchrono-wheel
python3 -m pip install --force-reinstall /tmp/metricchrono-wheel/metricchrono-0.1.0-*.whl
python3 -c "import metricchrono as mc; print(mc.tick_distance(1.2, mc.Tier(0.5, 1.0, 0.5, 1.0)))"
```

The wheel build runs Cargo and bundles the platform `metricchrono-ffi` shared
library. Source installs therefore require Cargo.

## Benchmarks

```sh
cargo bench -p metricchrono-core --bench clock_only_comparison
cargo bench -p metricchrono-core --bench ladder_throughput
```

The included benchmark is a deterministic synthetic guardrail: it verifies that
the tick ladder carries state-change signal that fixed-rate clock deltas cannot
carry. It is not a substitute for domain validation on customer data.
