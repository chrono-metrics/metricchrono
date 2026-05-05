# metricchrono-consensus

Thin Rust crate exposing MetricChrono's minimal consensus tick-field helpers:
`weighted_consensus`, `coherence_residual`, `coherence_residuals`,
`simple_weight_update`, and `ConsensusResult`.

The implementation is re-exported from `metricchrono-core` in v0.1.0 so
consensus behavior stays identical across the open-source crates.
