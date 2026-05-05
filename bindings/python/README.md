# MetricChrono Python Wrapper

This package is intentionally thin. It forwards calls through `ctypes` to the
same `metricchrono-ffi` native library used by the Rust and C APIs.

When building a wheel from the repository, the package build runs Cargo and
bundles the platform native library into the wheel:

```sh
python3 -m pip wheel bindings/python --no-deps -w /tmp/metricchrono-wheel
python3 -m pip install /tmp/metricchrono-wheel/metricchrono-0.1.0-*.whl
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

tier = mc.Tier(0.5, 1.0, 0.5, 1.0)
print(mc.tick_distance(1.2, tier))
```
