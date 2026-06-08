# Release Checklist

This checklist keeps publication mechanical and ordered. It is intentionally
separate from the implementation scope in `docs/scope.md`.

## Automated release (recommended)

`.github/workflows/release.yml` runs the whole release from a version tag. Bump
the version in all three manifests so they agree, update the changelog, then tag
and push:

```sh
# 1. bump the version in ALL of:
#      Cargo.toml ([workspace.package] version)
#      bindings/js/package.json
#      bindings/python/pyproject.toml
# 2. add the new dated section to CHANGELOG.md
git commit -am "Release X.Y.Z"
git tag -a vX.Y.Z -m "metricchrono X.Y.Z"
git push origin main vX.Y.Z
```

The workflow refuses to publish unless the tag matches all three manifests, the
tagged commit is an ancestor of `origin/main`, and the version is not already
published on any registry. It then re-runs `scripts/ci.sh` on the tagged commit,
publishes crates.io (`metricchrono-core` then `metricchrono-ffi`), PyPI
(`metricchrono`), and npm (`@metricchrono/core`), and cuts a GitHub release from
the matching changelog section. `workflow_dispatch` is **dry-run only** — it
validates the whole pipeline and publishes nothing (real publishes come from
tags):

```sh
gh workflow run release.yml
```

If a publish job fails partway (a registry hiccup, an index-propagation
timeout), fix the cause and **re-run the failed jobs** from the Actions run: the
publish steps are idempotent — already-published crates and versions are skipped
— so a re-run completes the partial release instead of erroring.

### One-time setup

- Repo secret `CARGO_REGISTRY_TOKEN` — a crates.io API token.
- An npm **trusted publisher** on `@metricchrono/core` (npmjs.com → the package →
  Settings → Trusted Publisher → GitHub Actions): organization `chrono-metrics`,
  repository `metricchrono`, workflow `release.yml`. OIDC, so no npm token is
  stored (the `publish-npm` job carries `id-token: write` and upgrades npm to a
  trusted-publishing-capable version).
- A PyPI **trusted publisher** on the `metricchrono` project (pypi.org → Your
  projects → metricchrono → Manage → Publishing → Add a new publisher → GitHub):
  owner `chrono-metrics`, repository `metricchrono`, workflow `release.yml`.
  OIDC, so no PyPI token is stored.

### Recommended hardening (optional)

- Add a protected `release` environment (Settings → Environments) with required
  reviewers and reference it from the publish jobs, so every real publish needs a
  human approval. Protect the `v*` tag pattern so only maintainers can push
  release tags.
- Drop the long-lived tokens by migrating npm and crates.io to **OIDC trusted
  publishing** (both now support it, as PyPI already does here) — then no
  `NPM_TOKEN` or `CARGO_REGISTRY_TOKEN` secret is stored at all.

The sections below are the manual fallback and the reference for what the
workflow automates.

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
python3 -m pip wheel /tmp/metricchrono-sdist/metricchrono-0.2.0.tar.gz --no-deps -w /tmp/metricchrono-wheel
python3 -m pip install --force-reinstall /tmp/metricchrono-wheel/metricchrono-0.2.0-*.whl
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
