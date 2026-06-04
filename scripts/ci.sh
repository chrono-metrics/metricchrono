#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

case "$(uname -s)" in
  Linux*)
    ffi_lib="target/release/libmetricchrono_ffi.so"
    ;;
  Darwin*)
    ffi_lib="target/release/libmetricchrono_ffi.dylib"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    ffi_lib="target/release/metricchrono_ffi.dll"
    ;;
  *)
    echo "unsupported OS for MetricChrono FFI library: $(uname -s)" >&2
    exit 1
    ;;
esac

tmp_dirs=()
cleanup() {
  if ((${#tmp_dirs[@]})); then
    rm -rf "${tmp_dirs[@]}"
  fi
}
trap cleanup EXIT

header() {
  printf '\n==> %s\n' "$1"
}

# A missing toolchain is a HARD ERROR under CI (so a workflow drift can never
# silently drop the JS/Python gate), but a graceful skip for local Rust-only runs.
missing_tool() { # $1=label $2=tool
  if [ -n "${CI:-}" ]; then
    echo "ERROR ${1}: ${2} not found but required in CI" >&2
    exit 1
  fi
  echo "SKIPPING ${1}: ${2} not found"
  return 1
}

has_js_tools() {
  local label="$1"
  command -v node >/dev/null 2>&1 || { missing_tool "$label" node; return 1; }
  command -v npm >/dev/null 2>&1 || { missing_tool "$label" npm; return 1; }
  return 0
}

has_python() {
  local label="$1"
  command -v python >/dev/null 2>&1 || { missing_tool "$label" python; return 1; }
  return 0
}

run_rust() {
  header "rust"

  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  cargo test --workspace --no-default-features
  RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
  cargo build -p metricchrono-ffi --release
  cmp -s include/metricchrono.h crates/metricchrono-ffi/include/metricchrono.h
  cargo bench --workspace --no-run
  cargo run -p metricchrono-core --example basic_tick
  cargo run -p metricchrono-core --example multiscale_ladder
  cargo run -p metricchrono-core --example smooth_surrogate
  cargo run -p metricchrono-core --example event_log
  cargo run -p metricchrono-core --example consensus_field
  cargo run -p metricchrono-core --example stream_monitor
}

run_js() {
  header "js"

  if ! has_js_tools "js"; then
    return 0
  fi

  (
    cd bindings/js
    npm test
  )
}

run_python() {
  header "python"

  if ! has_python "python"; then
    return 0
  fi

  python -m pip install build pytest
  PYTHONPATH=bindings/python METRICCHRONO_FFI_LIB="$ffi_lib" python -m pytest bindings/python/tests
  PYTHONPATH=bindings/python METRICCHRONO_FFI_LIB="$ffi_lib" python bindings/python/tests/golden.py
}

run_package() {
  header "package"

  cargo package -p metricchrono-core
  cargo package -p metricchrono-ffi --allow-dirty --no-verify --list

  if has_js_tools "package js"; then
    (
      cd bindings/js
      npm pack --dry-run
    )
  fi

  if has_python "package python"; then
    local sdist_dir wheel_dir
    sdist_dir="$(mktemp -d "${TMPDIR:-/tmp}/metricchrono-sdist.XXXXXX")"
    wheel_dir="$(mktemp -d "${TMPDIR:-/tmp}/metricchrono-wheel.XXXXXX")"
    tmp_dirs+=("$sdist_dir" "$wheel_dir")

    python -m build bindings/python --sdist --outdir "$sdist_dir"
    python -m pip wheel "$sdist_dir/metricchrono-0.1.0.tar.gz" --no-deps -w "$wheel_dir"
    python -m pip install --force-reinstall "$wheel_dir"/metricchrono-0.1.0-*.whl
    python -c "import metricchrono as mc; print(mc.tick_distance(1.2, mc.Tier(0.5, 1.0, 0.5, 1.0)))"
  fi
}

phase="${1:-all}"

case "$phase" in
  rust)
    run_rust
    ;;
  js)
    run_js
    ;;
  python)
    run_python
    ;;
  package)
    run_package
    ;;
  all)
    run_rust
    run_js
    run_python
    run_package
    ;;
  *)
    echo "usage: scripts/ci.sh [rust|js|python|package|all]" >&2
    exit 2
    ;;
esac
