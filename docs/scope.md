# Project Scope

MetricChrono publishes a compact, deterministic core for epsilon-delta-p tick
features and multiscale ladder representations.

In scope:

- the Rust core API, including event-log and consensus helpers;
- default metric examples (`Absolute`, `Euclidean`, `MetricFn`) and the
  feature-gated `metrics-extra` metric set;
- smooth tick surrogates;
- an in-memory event log;
- adaptive zoom helpers;
- minimal consensus helpers;
- C, Python, and JavaScript bindings; the C ABI exposes `mc_tick_distance` and
  `mc_tick_distance_raw`, not `mc_tick_distance_unchecked`;
- golden fixtures, examples, benchmarks, and documentation.

Out of scope for this repository:

- product-specific deployment tooling;
- hosted services;
- organization-specific integrations.

Keep new contributions focused on portable core behavior, stable bindings,
clear examples, and cross-language reproducibility.
