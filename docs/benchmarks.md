# Benchmarks

MetricChrono includes two dependency-free benchmark binaries:

- `clock_only_comparison` checks whether ladder ticks recover salient synthetic
  state changes when wall-clock deltas are nearly constant and therefore weakly
  informative.
- `ladder_throughput` measures repeated ladder evaluation throughput for a
  32-tier geometric ladder.

Run them from the repository root:

```sh
cargo bench -p metricchrono-core --bench clock_only_comparison
cargo bench -p metricchrono-core --bench ladder_throughput
```

The comparison benchmark is intentionally synthetic and deterministic. It is a
release guardrail for the public primitive, not a domain-performance claim.
Production validation should use domain-specific metrics, calibration, and
historical data outside this open-core repository.
