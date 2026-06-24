# MetricChrono Python

Production Python bindings for MetricChrono. The PyPI package uses the bundled `metricchrono-ffi` native library via `ctypes` when it is present, and otherwise falls back to a byte-identical pure-Python implementation — so `import metricchrono` works with no native library at all.

## Install

```sh
python3 -m pip install metricchrono
```

`pip install metricchrono` downloads a pre-built binary wheel and needs no Rust toolchain or Cargo. Wheels are published for Linux (manylinux x86_64 and aarch64), macOS (Intel and Apple Silicon), and Windows (x86_64).

Only the source-distribution (sdist) fallback build needs Cargo plus a Rust toolchain — used when pip cannot find a matching wheel for the target platform.

## Source-tree usage

To use the wrapper directly from a checkout without installing a wheel, no native build is required: the pure-Python backend works from the source tree. Build the native library from the repository root only when you want the fast path, then point Python at it:

```sh
cargo build -p metricchrono-ffi --release
export METRICCHRONO_FFI_LIB=target/release/libmetricchrono_ffi.dylib
```

Use `.so` on Linux and `.dll` on Windows.

Backend selection defaults to `METRICCHRONO_BACKEND=auto`: Python uses the
native library when the wheel or source-tree cdylib is present and loadable,
otherwise it falls back to the pure-Python backend. Force a backend with
`METRICCHRONO_BACKEND=python` or `METRICCHRONO_BACKEND=native`.

## Example

```python
import metricchrono as mc

ladder = mc.geometric_ladder(0.5, 1.0, 2.0, 4, 0.5, 1.0)
print(mc.ladder_distance(3.0, ladder))

with mc.EventLog(2) as log:
    log.append(10, [1.0, 0.0])
    log.append(11, [1.0, 1.0])
    print(log.next_event(0, 0))
```

## API Surface

- Kernel and ladders: `Tier`, `tick_distance`, `ladder_distance`,
  `geometric_ladder`, `Ladder`.
- Serialization: `tier_from_schema`, `tier_to_schema`, `ladder_from_schema`,
  `ladder_to_schema`, `tick_vector_from_schema`, `tick_vector_to_schema`,
  `consensus_result_from_schema`.
- Smooth and adaptive helpers: `smooth_tick_distance`,
  `smooth_ladder_distance`, `adaptive_ladder_distance`, `ZoomDecision`.
- Event memory: `EventLog`.
- Consensus: `weighted_consensus`, `coherence_residuals`,
  `simple_weight_update`.
