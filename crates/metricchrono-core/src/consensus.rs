use crate::ladder::{ensure_output, ensure_shape, sanitize_signed};
use crate::{MetricChronoError, Result};

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
