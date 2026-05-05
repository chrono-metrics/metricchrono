# Benchmark Baseline v0.1.0

This baseline is a local release guardrail, not a public performance claim.

- Date: 2026-05-05
- Machine: Apple M1 Pro
- OS: macOS 15.7.3
- Rust: rustc 1.95.0 (59807616e 2026-04-14), LLVM 22.1.3
- Profile: Cargo `bench` profile, default repository settings
- Command: `cargo bench -p metricchrono-core --bench publish_suite`

```text
single tick throughput: 1000000 iterations in 20.925875ms (20.9 ns/eval)
ladder throughput 4 tiers: 250000 iterations in 22.007125ms (88.0 ns/eval)
ladder throughput 8 tiers: 250000 iterations in 43.775708ms (175.1 ns/eval)
ladder throughput 16 tiers: 250000 iterations in 64.219167ms (256.9 ns/eval)
ladder throughput 32 tiers: 250000 iterations in 97.157625ms (388.6 ns/eval)
smooth tick throughput: 250000 iterations in 29.576083ms (118.3 ns/eval)
event-log append/next_event: 50000 iterations in 2.551084ms (51.0 ns/eval)
consensus 16 sources x 8 tiers: 100000 iterations in 41.82175ms (418.2 ns/eval)
```

JavaScript package dry-run:

- Command: `npm pack --dry-run --json` from `bindings/js`
- Tarball: `metricchrono-core-0.1.0.tgz`
- Packed size: 6242 bytes
- Unpacked size: 26393 bytes
