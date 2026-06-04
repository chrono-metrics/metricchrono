# MetricChrono Python

Production Python bindings for the same `metricchrono-ffi` native library used
by the Rust and C APIs.

When building a wheel from the repository, the package build runs Cargo and
bundles the platform native library into the wheel:

```sh
python3 -m pip wheel bindings/python --no-deps -w /tmp/metricchrono-wheel
python3 -m pip install /tmp/metricchrono-wheel/metricchrono-0.2.0-*.whl
```

Cargo must be available for source builds. To use the wrapper directly from the
source tree without installing a wheel, build the native library from the
repository root:

```sh
cargo build -p metricchrono-ffi --release
```

Then point Python at the library if it is not installed in a standard dynamic
loader path:

```sh
export METRICCHRONO_FFI_LIB=target/release/libmetricchrono_ffi.dylib
```

Use `.so` on Linux and `.dll` on Windows.

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

## Verify

```sh
PYTHONPATH=bindings/python METRICCHRONO_FFI_LIB=target/release/libmetricchrono_ffi.dylib \
  python3 bindings/python/tests/golden.py
python3 -m pip install build
python3 -m build bindings/python --sdist --outdir /tmp/metricchrono-sdist
python3 -m pip wheel /tmp/metricchrono-sdist/metricchrono-0.2.0.tar.gz --no-deps -w /tmp/metricchrono-wheel
```
