# C ABI

The `metricchrono-ffi` crate exposes a stable C-compatible boundary over the
core functions.

Build:

```sh
cargo build -p metricchrono-ffi --release
```

Core symbols:

- `mc_tick_distance`
- `mc_ladder_distance`
- `mc_smooth_tick_distance`
- `mc_geometric_ladder`
- `mc_weighted_consensus`
- `mc_simple_weight_update`
- `mc_event_log_new`
- `mc_event_log_free`
- `mc_event_log_append`
- `mc_event_log_next_event`
- `mc_event_log_len`

The public C declarations live in [`include/metricchrono.h`](../include/metricchrono.h).

All functions return `MCStatus`. Output buffers are caller-owned. Passing a
buffer shorter than the documented length returns `MC_STATUS_BUFFER_TOO_SMALL`.
