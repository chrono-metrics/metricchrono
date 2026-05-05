# MetricChrono Python Wrapper

This package is intentionally thin. It loads the `metricchrono-ffi` shared
library and forwards calls through `ctypes`.

Build the native library from the repository root:

```sh
cargo build -p metricchrono-ffi --release
```

Then point Python at the library if it is not installed in a standard dynamic
loader path:

```sh
export METRICCHRONO_FFI_LIB=target/release/libmetricchrono_ffi.dylib
```

Use `.so` on Linux and `.dll` on Windows.
