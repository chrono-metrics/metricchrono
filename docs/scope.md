# Project Scope

MetricChrono publishes a compact, deterministic core for epsilon-delta-p tick
features and multiscale ladder representations.

In scope:

- the Rust core API;
- thin Rust crate boundaries for event-log and consensus users;
- basic metric examples;
- smooth tick surrogates;
- an in-memory event log;
- adaptive zoom helpers;
- minimal consensus helpers;
- C, Python, and JavaScript bindings;
- golden fixtures, examples, benchmarks, and documentation.

Out of scope for this repository:

- product-specific deployment tooling;
- hosted services;
- organization-specific integrations.

Keep new contributions focused on portable core behavior, stable bindings,
clear examples, and cross-language reproducibility.
