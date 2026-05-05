# metricchrono-log

Thin Rust crate exposing MetricChrono's basic in-memory event log:
`EventId`, `EventLog`, `EventRecord`, `EventSummary`, and `TierEventIter`.

The implementation is re-exported from `metricchrono-core` in v0.1.0 so event
memory behavior stays identical across the open-source crates.
