use crate::ladder::{ensure_output, finite_or_max, sanitize_distance, validate_ladder};
use crate::{Result, Tier};

/// Differentiable surrogate for the epsilon-delta-p tick.
///
/// `sharpness` controls the logistic approximation. Values around `10.0`
/// behave close to the hard gate while still remaining smooth.
pub fn smooth_tick_distance(distance: f64, tier: Tier, sharpness: f64) -> Result<f64> {
    tier.validate_at(0)?;
    if !sharpness.is_finite() || sharpness <= 0.0 {
        return Err(crate::MetricChronoError::InvalidArgument(
            "sharpness must be finite and > 0",
        ));
    }

    let d = sanitize_distance(distance);
    let gate = sigmoid(sharpness * (d - tier.epsilon));
    let x = d / tier.delta;
    let stair = smooth_stair(x, sharpness);
    Ok(finite_or_max(tier.gain() * gate * stair))
}

/// Fill `out` with smooth tick values.
pub fn smooth_ladder_distance(
    distance: f64,
    ladder: &[Tier],
    sharpness: f64,
    out: &mut [f64],
) -> Result<()> {
    validate_ladder(ladder)?;
    ensure_output(ladder.len(), out.len())?;
    for (slot, tier) in out.iter_mut().zip(ladder.iter().copied()) {
        *slot = smooth_tick_distance(distance, tier, sharpness)?;
    }
    Ok(())
}

/// Allocate and return smooth tick values.
pub fn smooth_ladder_values(distance: f64, ladder: &[Tier], sharpness: f64) -> Result<Vec<f64>> {
    let mut out = vec![0.0; ladder.len()];
    smooth_ladder_distance(distance, ladder, sharpness, &mut out)?;
    Ok(out)
}

fn sigmoid(value: f64) -> f64 {
    let clipped = value.clamp(-60.0, 60.0);
    1.0 / (1.0 + (-clipped).exp())
}

fn smooth_stair(x: f64, sharpness: f64) -> f64 {
    if x <= 0.0 {
        return sigmoid(sharpness * x);
    }
    let hard = x.ceil();
    if !hard.is_finite() || hard > 4096.0 {
        return hard;
    }
    let j_max = hard as usize + 1;
    1.0 + (1..=j_max)
        .map(|j| sigmoid(sharpness * (x - j as f64)))
        .sum::<f64>()
}
