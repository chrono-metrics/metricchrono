# C ABI

The `metricchrono-ffi` crate exposes a stable C-compatible boundary over the
core functions.

Build:

```sh
cargo build -p metricchrono-ffi --release
```

The public C declarations in [`include/metricchrono.h`](../include/metricchrono.h)
are the source of truth for exported symbols.

Most functions return `MCStatus`. Output buffers are caller-owned. Passing a
buffer shorter than the documented length returns `MC_STATUS_BUFFER_TOO_SMALL`.

`mc_coverage_meter_new_with_callback` registers a caller-supplied
`MCDistanceFn`; the callback must not unwind, `user_data` must outlive the
meter, and returning NaN rejects admission (the safe failure mode).

`MCLadder`, `MCEventLog`, and `MCCoverageMeter` are opaque handles owned by the caller. Every
successful `mc_ladder_new` must be paired with `mc_ladder_free`, and every
successful `mc_event_log_new` must be paired with `mc_event_log_free`, and every successful coverage-meter constructor with `mc_coverage_meter_free`.

No Rust panic is allowed to cross the C ABI boundary. C-callable functions catch
panics and report `MC_STATUS_PANIC`. The C ABI stores no global mutable state;
separate handles can be used independently, but concurrent access to the same
handle must be synchronized by the caller.
