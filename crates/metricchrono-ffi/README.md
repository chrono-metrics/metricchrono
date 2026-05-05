# metricchrono-ffi

C ABI for `metricchrono-core`. This crate exposes plain structs, caller-owned
buffers, and status codes so the public kernel can be embedded from C, C++,
Python `ctypes`, and other FFI consumers without depending on Rust generics.

The header is available at `include/metricchrono.h` in this crate and at the
repository root for C/C++ consumers.

## Build

```sh
cargo build -p metricchrono-ffi --release
```

The build emits the platform library in `target/release`:

- macOS: `libmetricchrono_ffi.dylib`
- Linux: `libmetricchrono_ffi.so`
- Windows: `metricchrono_ffi.dll`

## API Shape

Core functions return `MCStatus` and write results into caller-owned output
buffers:

- `mc_tick_distance`
- `mc_ladder_distance`
- `mc_adaptive_ladder_distance`
- `mc_smooth_tick_distance`
- `mc_geometric_ladder`
- `mc_weighted_consensus`
- `mc_simple_weight_update`
- `mc_event_log_new`, `mc_event_log_append`, `mc_event_log_next_event`,
  `mc_event_log_len`, and `mc_event_log_free`

Invalid pointers return `MC_STATUS_NULL`, short buffers return
`MC_STATUS_BUFFER_TOO_SMALL`, invalid inputs return
`MC_STATUS_INVALID_ARGUMENT`, and caught Rust panics return `MC_STATUS_PANIC`.

## Publish Order

Publish `metricchrono-core` first. This crate depends on
`metricchrono-core = "0.1.0"`, so a standalone `cargo publish --dry-run -p
metricchrono-ffi` cannot resolve until the core crate is already available in
the crates.io index.
