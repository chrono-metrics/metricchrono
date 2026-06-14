//! Open-source MetricChrono core.
//!
//! This crate contains the public primitive: single-scale ticks, multiscale
//! ladders, basic metric traits, a smooth surrogate, a basic event log,
//! adaptive zoom helpers, minimal consensus utilities, the per-tier coverage
//! meter (throughput's complementary read-out), and guarded ambient CFAR
//! estimators.

mod ambient;
mod comparator;
mod consensus;
mod coverage;
mod error;
mod event_log;
mod geometry;
mod ladder;
mod metrics;
mod orientation;
mod psychophysics;
mod rate_distortion;
mod schema;
mod session;
mod smooth;
mod tier;
mod zoom;

pub use ambient::{CfarDecision, GuardedAmbientScale, GuardedQuantileThreshold};
pub use comparator::{
    factored_threshold, kappa_pq, miss_probability_bound, sparse_dominance_boundary,
    threshold_ratio, unified_threshold,
};
pub use consensus::{
    coherence_residual, coherence_residuals, simple_weight_update, tier_residuals,
    weighted_consensus, weighted_consensus_tierwise, ConsensusInput, ConsensusResult, SourceTick,
};
pub use coverage::{classify_regime, progress_efficiency, CoverageMeter, OperatingRegime};
pub use error::{MetricChronoError, Result};
pub use event_log::{EventId, EventLog, EventRecord, EventSummary, TierEventIter};
pub use geometry::{
    branching_number, comparison_angle, comparison_angle_unchecked, greedy_packing, radial_sort,
    shell_index, shell_indices,
};
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
pub use orientation::{discrete_derivative, earth_mover_1d, entropy_openness};
pub use psychophysics::{
    aggregate_slope, compressed_readout, log_bisection_pse, magnitude_bisection_pse,
    vierordt_crossover,
};
pub use rate_distortion::{geometric_boundaries, harmonic_mean_representative, staircase_penalty_ratio};
pub use schema::{ConsensusResultDocument, LadderDocument, TickVectorDocument, TierDocument};
pub use session::{Session, StepResult};
pub use smooth::{
    smooth_ladder_distance, smooth_ladder_values, smooth_tick_distance, SmoothParams,
};
pub use tier::{Tier, TierBuilder};
pub use zoom::{
    adaptive_ladder_distance, adaptive_zoom_window, zoom_ladder_distance, ZoomDecision, ZoomPolicy,
};
