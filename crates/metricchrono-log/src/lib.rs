//! Event-log crate for MetricChrono.
//!
//! The v0.1 implementation is single-sourced in `metricchrono-core`; this crate
//! provides the publication boundary for users who only need event memory.

pub use metricchrono_core::{EventLog, EventRecord, EventSummary, TierEventIter};
