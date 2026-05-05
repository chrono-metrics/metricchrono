//! Consensus crate for MetricChrono.
//!
//! The v0.1 implementation is single-sourced in `metricchrono-core`; this crate
//! provides the publication boundary for users who only need consensus helpers.

pub use metricchrono_core::{
    coherence_residual, coherence_residuals, simple_weight_update, weighted_consensus,
    ConsensusResult, ConsensusResultDocument,
};
