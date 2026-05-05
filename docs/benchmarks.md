# Benchmarks

MetricChrono includes dependency-free benchmark binaries:

- `clock_only_comparison` checks whether ladder ticks recover salient synthetic
  state changes when wall-clock deltas are nearly constant and therefore weakly
  informative.
- `ladder_throughput` measures repeated ladder evaluation throughput for a
  32-tier geometric ladder.
- `publish_suite` covers the v0.1 public release guardrails: single tick,
  ladder throughput for 4/8/16/32 tiers, smooth tick, event-log
  append/next_event, and consensus over 16 sources x 8 tiers.

Run them from the repository root:

```sh
cargo bench -p metricchrono-core --bench clock_only_comparison
cargo bench -p metricchrono-core --bench ladder_throughput
cargo bench -p metricchrono-core --bench publish_suite
```

The comparison benchmark is intentionally synthetic and deterministic. It is a
release guardrail for the public primitive, not a domain-performance claim.
Production validation should use domain-specific metrics, calibration, and
historical data outside this open-core repository.

CI compiles every benchmark with:

```sh
cargo bench --workspace --no-run
```

The v0.1.0 local baseline is stored in
[`docs/benchmark-baseline-v0.1.0.md`](benchmark-baseline-v0.1.0.md).
