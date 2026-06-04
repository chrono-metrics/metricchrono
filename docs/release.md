# Release Checklist

This checklist keeps publication mechanical and ordered. It is intentionally
separate from the implementation scope in `docs/scope.md`.

## Repository

Before making the repository public:

```sh
git status --short --branch
git diff --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo bench --workspace --no-run
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
because `metricchrono-ffi` depends on `metricchrono-core`.

## JavaScript

```sh
npm test --prefix bindings/js -- golden
npm pack --dry-run --prefix bindings/js
```

The package currently publishes as `@metricchrono/core`.

## Python

Build and verify the source distribution and platform wheel from the repository
root:

```sh
python3 -m pip install build
python3 -m pip install pytest
PYTHONPATH=bindings/python METRICCHRONO_FFI_LIB=target/release/libmetricchrono_ffi.dylib \
  python3 -m pytest bindings/python/tests
python3 -m build bindings/python --sdist --outdir /tmp/metricchrono-sdist
python3 -m pip wheel /tmp/metricchrono-sdist/metricchrono-0.1.0.tar.gz --no-deps -w /tmp/metricchrono-wheel
python3 -m pip install --force-reinstall /tmp/metricchrono-wheel/metricchrono-0.1.0-*.whl
python3 -c "import metricchrono as mc; print(mc.tick_distance(1.2, mc.Tier(0.5, 1.0, 0.5, 1.0)))"
```

The source distribution vendors the Rust workspace needed by the Python build.
The wheel build runs Cargo and bundles the platform `metricchrono-ffi` shared
library. Source installs therefore require Cargo and a Rust toolchain.

## Benchmarks

```sh
cargo bench -p metricchrono-core --bench clock_only_comparison
cargo bench -p metricchrono-core --bench ladder_throughput
cargo bench -p metricchrono-core --bench publish_suite
```

The included benchmarks are deterministic release guardrails. The clock-only
comparison verifies that
the tick ladder carries state-change signal that fixed-rate clock deltas cannot
carry in a synthetic regime. They are not a substitute for domain validation on
customer data, and no public nanosecond claim should be made without machine,
compiler, flags, and method details.
