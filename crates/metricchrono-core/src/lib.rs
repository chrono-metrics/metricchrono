//! Open-source MetricChrono core.
//!
//! This crate contains the public primitive: single-scale ticks, multiscale
//! ladders, basic metric traits, a smooth surrogate, a basic event log,
//! adaptive zoom helpers, minimal consensus utilities, the per-tier coverage
//! meter (throughput's complementary read-out), and guarded ambient CFAR
//! estimators.

mod ambient;
mod consensus;
mod coverage;
mod error;
mod event_log;
mod ladder;
mod metrics;
mod schema;
mod smooth;
mod tier;
mod zoom;

pub use ambient::{CfarDecision, GuardedAmbientScale, GuardedQuantileThreshold};
pub use consensus::{
    coherence_residual, coherence_residuals, simple_weight_update, tier_residuals,
    weighted_consensus, weighted_consensus_tierwise, ConsensusInput, ConsensusResult, SourceTick,
};
pub use coverage::{classify_regime, progress_efficiency, CoverageMeter, OperatingRegime};
pub use error::{MetricChronoError, Result};
pub use event_log::{EventId, EventLog, EventRecord, EventSummary, TierEventIter};
pub use ladder::{
    carry_rules, custom_ladder, geometric_ladder, ladder_distance, ladder_values, normalize_ticks,
    tick_distance, try_tick_distance, validate_ladder, Ladder, Normalization, PromotionCounter,
    TickVector,
};
pub use metrics::{ladder_pair, tick_pair, Absolute, Euclidean, Metric, MetricFn};
#[cfg(feature = "metrics-extra")]
pub use metrics::{
    Cosine, DiagonalMahalanobis, JensenShannon, KullbackLeibler, Manhattan, SquaredEuclidean,
};
pub use schema::{ConsensusResultDocument, LadderDocument, TickVectorDocument, TierDocument};
pub use smooth::{
    smooth_ladder_distance, smooth_ladder_values, smooth_tick_distance, SmoothParams,
};
pub use tier::{Tier, TierBuilder};
pub use zoom::{
    adaptive_ladder_distance, adaptive_zoom_window, zoom_ladder_distance, ZoomDecision, ZoomPolicy,
};
