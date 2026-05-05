use crate::ladder::{ensure_output, ensure_shape, sanitize_signed};
use crate::{MetricChronoError, Result};

/// One source-local tick vector.
pub type SourceTick<'a> = &'a [f64];

/// Source x tier inputs for tier-wise weighted consensus.
#[derive(Clone, Copy, Debug)]
pub struct ConsensusInput<'a> {
    pub tick_vectors: &'a [SourceTick<'a>],
    pub tier_weights: &'a [SourceTick<'a>],
}

/// Metadata returned by weighted consensus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConsensusResult {
    pub sources: usize,
    pub tiers: usize,
    pub total_weight: f64,
}

/// Compute a weighted consensus tick vector.
pub fn weighted_consensus(
    tick_vectors: &[&[f64]],
    weights: &[f64],
    out: &mut [f64],
) -> Result<ConsensusResult> {
    if tick_vectors.is_empty() {
        return Err(MetricChronoError::InvalidArgument(
            "at least one source is required",
        ));
    }
    ensure_shape(tick_vectors.len(), weights.len(), "source weights")?;
    let tiers = tick_vectors[0].len();
    if tiers == 0 {
        return Err(MetricChronoError::EmptyLadder);
    }
    ensure_output(tiers, out.len())?;
    for vector in tick_vectors {
        ensure_shape(tiers, vector.len(), "tick vector")?;
    }

    let mut total_weight = 0.0;
    out[..tiers].fill(0.0);
    for (vector, weight) in tick_vectors.iter().zip(weights.iter().copied()) {
        if !weight.is_finite() || weight < 0.0 {
            return Err(MetricChronoError::InvalidArgument(
                "weights must be finite and >= 0",
            ));
        }
        if weight == 0.0 {
            continue;
        }
        total_weight += weight;
        for (slot, value) in out.iter_mut().zip(vector.iter().copied()) {
            *slot += weight * sanitize_signed(value);
        }
    }
    if total_weight <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "total consensus weight must be > 0",
        ));
    }
    for value in &mut out[..tiers] {
        *value /= total_weight;
    }
    Ok(ConsensusResult {
        sources: tick_vectors.len(),
        tiers,
        total_weight,
    })
}

/// Compute a weighted consensus tick vector with source x tier weights.
pub fn weighted_consensus_tierwise(
    input: ConsensusInput<'_>,
    out: &mut [f64],
) -> Result<ConsensusResult> {
    if input.tick_vectors.is_empty() {
        return Err(MetricChronoError::InvalidArgument(
            "at least one source is required",
        ));
    }
    ensure_shape(
        input.tick_vectors.len(),
        input.tier_weights.len(),
        "source tier weights",
    )?;
    let tiers = input.tick_vectors[0].len();
    if tiers == 0 {
        return Err(MetricChronoError::EmptyLadder);
    }
    ensure_output(tiers, out.len())?;
    out[..tiers].fill(0.0);

    let mut totals = vec![0.0; tiers];
    for (source, weights) in input.tick_vectors.iter().zip(input.tier_weights) {
        ensure_shape(tiers, source.len(), "tick vector")?;
        ensure_shape(tiers, weights.len(), "tier weights")?;
        for tier in 0..tiers {
            let weight = weights[tier];
            if !weight.is_finite() || weight < 0.0 {
                return Err(MetricChronoError::InvalidArgument(
                    "weights must be finite and >= 0",
                ));
            }
            if weight == 0.0 {
                continue;
            }
            totals[tier] += weight;
            out[tier] += weight * sanitize_signed(source[tier]);
        }
    }

    for (tier, total) in totals.iter().copied().enumerate() {
        if total <= 0.0 {
            return Err(MetricChronoError::InvalidArgument(
                "total consensus weight must be > 0 for every tier",
            ));
        }
        out[tier] /= total;
    }

    Ok(ConsensusResult {
        sources: input.tick_vectors.len(),
        tiers,
        total_weight: totals.iter().sum(),
    })
}

/// Root-mean-square residual between one source vector and consensus.
pub fn coherence_residual(source_tick: &[f64], consensus: &[f64]) -> Result<f64> {
    ensure_shape(consensus.len(), source_tick.len(), "coherence residual")?;
    if consensus.is_empty() {
        return Err(MetricChronoError::EmptyLadder);
    }
    let mse = source_tick
        .iter()
        .zip(consensus)
        .map(|(source, center)| {
            let diff = sanitize_signed(*source) - sanitize_signed(*center);
            diff * diff
        })
        .sum::<f64>()
        / consensus.len() as f64;
    Ok(mse.sqrt())
}

/// Compute per-tier absolute residuals for one source vector.
pub fn tier_residuals(source_tick: &[f64], consensus: &[f64], out: &mut [f64]) -> Result<()> {
    ensure_shape(consensus.len(), source_tick.len(), "tier residuals")?;
    if consensus.is_empty() {
        return Err(MetricChronoError::EmptyLadder);
    }
    ensure_output(consensus.len(), out.len())?;
    for (slot, (source, center)) in out.iter_mut().zip(source_tick.iter().zip(consensus)) {
        *slot = (sanitize_signed(*source) - sanitize_signed(*center)).abs();
    }
    Ok(())
}

/// Compute residuals for all sources.
pub fn coherence_residuals(
    tick_vectors: &[&[f64]],
    consensus: &[f64],
    out: &mut [f64],
) -> Result<()> {
    ensure_output(tick_vectors.len(), out.len())?;
    for (slot, vector) in out.iter_mut().zip(tick_vectors.iter().copied()) {
        *slot = coherence_residual(vector, consensus)?;
    }
    Ok(())
}

/// Update source weights from residuals and normalize in place.
pub fn simple_weight_update(
    weights: &mut [f64],
    residuals: &[f64],
    learning_rate: f64,
    floor: f64,
) -> Result<()> {
    ensure_shape(weights.len(), residuals.len(), "weight residuals")?;
    if weights.is_empty() {
        return Err(MetricChronoError::InvalidArgument(
            "at least one weight is required",
        ));
    }
    if !learning_rate.is_finite() || learning_rate < 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "learning_rate must be finite and >= 0",
        ));
    }
    if !floor.is_finite() || floor < 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "floor must be finite and >= 0",
        ));
    }

    let mut total = 0.0;
    for (weight, residual) in weights.iter_mut().zip(residuals.iter().copied()) {
        if !weight.is_finite() || *weight < 0.0 || !residual.is_finite() || residual < 0.0 {
            return Err(MetricChronoError::InvalidArgument(
                "weights and residuals must be finite and >= 0",
            ));
        }
        *weight = (*weight * (-learning_rate * residual).exp()).max(floor);
        total += *weight;
    }
    if total <= 0.0 {
        let uniform = 1.0 / weights.len() as f64;
        weights.fill(uniform);
        return Ok(());
    }
    for weight in weights {
        *weight /= total;
    }
    Ok(())
}
