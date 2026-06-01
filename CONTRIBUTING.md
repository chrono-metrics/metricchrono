# Contributing

MetricChrono is intentionally small. Contributions should keep the Rust core
deterministic, dependency-light, and easy to bind from other languages.

Before opening a pull request, run the same verification entrypoint CI uses:

```sh
scripts/ci.sh
```

For a faster Rust-only gate while iterating, run:

```sh
scripts/ci.sh rust
```

CI runs the same script, so local verification and CI stay in parity. To opt in
to the pre-push hook that runs the Rust gate before pushing, run:

```sh
git config core.hooksPath .githooks
```

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
