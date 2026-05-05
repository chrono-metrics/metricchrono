# Contributing

MetricChrono is intentionally small. Contributions should keep the Rust core
deterministic, dependency-light, and easy to bind from other languages.

Before opening a pull request, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
cargo build -p metricchrono-ffi --release
(cd bindings/js && npm test)
PYTHONPATH=bindings/python METRICCHRONO_FFI_LIB=target/release/libmetricchrono_ffi.dylib python3 bindings/python/tests/golden.py
```

On Linux, use `target/release/libmetricchrono_ffi.so` for
`METRICCHRONO_FFI_LIB`.

## Golden Vectors

Rust, Python, and JS must continue to pass the shared fixtures in
`crates/metricchrono-core/fixtures/`. When boundary behavior changes, update the
fixtures in the same patch as the implementation and explain the compatibility
impact.

## Scope

Keep contributions focused on portable core behavior, stable bindings, clear
examples, and cross-language reproducibility. Product-specific deployment
tooling, hosted services, and organization-specific integrations are out of
scope for this repository.
