# Project Scope

MetricChrono publishes a compact, deterministic core for epsilon-delta-p tick
features and multiscale ladder representations.

In scope:

- the aligned Rust-default, C ABI, Python, and JavaScript keep-set: tick
  distance helpers, tier and ladder construction, ladder validation, ladder
  distance/value helpers, normalization and carry rules, event-log navigation,
  smooth distances, adaptive ladder distance, weighted consensus, promotion
  counters, structured errors, and versioned schema helpers;
- default metric examples (`Absolute`, `Euclidean`, `MetricFn`) plus
  `tick_pair` and `ladder_pair`;
- the Rust-only `metrics-extra` feature for `Cosine`, `KullbackLeibler`,
  `JensenShannon`, `Manhattan`, `SquaredEuclidean`, and
  `DiagonalMahalanobis`; these six metrics are absent from the default
  bindings;
- smooth tick surrogates;
- an in-memory event log;
- adaptive zoom helpers;
- minimal consensus helpers;
- C, Python, and JavaScript bindings over the same default keep-set;
- golden fixtures, examples, benchmarks, and documentation.

`EventLog` follows an append-per-timestamp contract. Call `append` once for
each observation, including quiet all-zero tick records. Events are the
positive-tick subset of the appended records; `first_event(tier)` is the
tier-local chain head, and callers walk the chain with `next_event` or compact
summary/record readers.

Out of scope for this repository:

- product-specific deployment tooling;
- hosted services;
- organization-specific integrations.

Keep new contributions focused on portable core behavior, stable bindings,
clear examples, and cross-language reproducibility.
