use std::ops::Range;

use crate::ladder::{ensure_distance, ensure_output, tick_distance, validate_ladder};
use crate::{Result, Tier};

/// Result metadata for early-stop ladder evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZoomDecision {
    pub evaluated_tiers: usize,
    pub first_inactive_tier: Option<usize>,
    pub stopped_early: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZoomPolicy {
    FixedDepth(usize),
    EarlyStop,
    DepthCap(usize),
}

pub fn zoom_ladder_distance(
    distance: f64,
    ladder: &[Tier],
    policy: ZoomPolicy,
    out: &mut [f64],
) -> Result<ZoomDecision> {
    validate_ladder(ladder)?;
    ensure_distance(distance)?;
    ensure_output(ladder.len(), out.len())?;

    match policy {
        ZoomPolicy::FixedDepth(depth) => {
            if depth > ladder.len() {
                return Err(crate::MetricChronoError::InvalidArgument(
                    "fixed depth exceeds ladder length",
                ));
            }
            out[..ladder.len()].fill(0.0);
            for index in 0..depth {
                out[index] = tick_distance(distance, ladder[index]);
            }
            Ok(ZoomDecision {
                evaluated_tiers: depth,
                first_inactive_tier: None,
                stopped_early: false,
            })
        }
        ZoomPolicy::EarlyStop => adaptive_ladder_distance(distance, ladder, out),
        ZoomPolicy::DepthCap(depth_cap) => {
            if depth_cap > ladder.len() {
                return Err(crate::MetricChronoError::InvalidArgument(
                    "depth cap exceeds ladder length",
                ));
            }
            out[..ladder.len()].fill(0.0);
            for index in 0..depth_cap {
                if distance < ladder[index].epsilon {
                    return Ok(ZoomDecision {
                        evaluated_tiers: index + 1,
                        first_inactive_tier: Some(index),
                        stopped_early: true,
                    });
                }
                out[index] = tick_distance(distance, ladder[index]);
            }
            Ok(ZoomDecision {
                evaluated_tiers: depth_cap,
                first_inactive_tier: None,
                stopped_early: false,
            })
        }
    }
}

/// Compute a ladder vector and stop once sorted tiers are guaranteed inactive.
pub fn adaptive_ladder_distance(
    distance: f64,
    ladder: &[Tier],
    out: &mut [f64],
) -> Result<ZoomDecision> {
    validate_ladder(ladder)?;
    ensure_distance(distance)?;
    ensure_output(ladder.len(), out.len())?;

    let d = distance;
    for (index, tier) in ladder.iter().copied().enumerate() {
        if d < tier.epsilon {
            out[index..ladder.len()].fill(0.0);
            return Ok(ZoomDecision {
                evaluated_tiers: index + 1,
                first_inactive_tier: Some(index),
                stopped_early: true,
            });
        }
        out[index] = tick_distance(d, tier);
    }
    Ok(ZoomDecision {
        evaluated_tiers: ladder.len(),
        first_inactive_tier: None,
        stopped_early: false,
    })
}

/// Return a small tier range around the coarsest active tier.
pub fn adaptive_zoom_window(
    distance: f64,
    ladder: &[Tier],
    radius: usize,
) -> Result<Option<Range<usize>>> {
    validate_ladder(ladder)?;
    ensure_distance(distance)?;
    let d = distance;
    let Some(center) = ladder.iter().rposition(|tier| d >= tier.epsilon) else {
        return Ok(None);
    };
    let start = center.saturating_sub(radius);
    let end = (center + radius + 1).min(ladder.len());
    Ok(Some(start..end))
}
