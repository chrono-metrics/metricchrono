use crate::ladder::{ensure_distance, ensure_output, finite_or_max, validate_ladder};
use crate::{Result, Tier};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothParams {
    pub gate_sharpness: f64,
    pub stair_sharpness: f64,
    pub max_stairs: usize,
}

impl SmoothParams {
    pub fn new(gate_sharpness: f64, stair_sharpness: f64, max_stairs: usize) -> Result<Self> {
        let params = Self {
            gate_sharpness,
            stair_sharpness,
            max_stairs,
        };
        params.validate()?;
        Ok(params)
    }

    pub fn sharpness(sharpness: f64) -> Result<Self> {
        Self::new(sharpness, sharpness, 4096)
    }

    fn validate(self) -> Result<()> {
        if !self.gate_sharpness.is_finite() || self.gate_sharpness <= 0.0 {
            return Err(crate::MetricChronoError::InvalidArgument(
                "gate_sharpness must be finite and > 0",
            ));
        }
        if !self.stair_sharpness.is_finite() || self.stair_sharpness <= 0.0 {
            return Err(crate::MetricChronoError::InvalidArgument(
                "stair_sharpness must be finite and > 0",
            ));
        }
        if self.max_stairs == 0 {
            return Err(crate::MetricChronoError::InvalidArgument(
                "max_stairs must be > 0",
            ));
        }
        Ok(())
    }
}

/// Differentiable surrogate for the epsilon-delta-p tick.
///
/// Sharpness controls the logistic approximation. Values around `10.0` behave
/// close to the hard gate while still remaining smooth.
pub fn smooth_tick_distance(distance: f64, tier: Tier, params: SmoothParams) -> Result<f64> {
    tier.validate_at(0)?;
    params.validate()?;
    ensure_distance(distance)?;

    let d = distance;
    let gate = sigmoid(params.gate_sharpness * (d - tier.epsilon));
    let x = d / tier.delta;
    let stair = smooth_stair(x, params.stair_sharpness, params.max_stairs);
    Ok(finite_or_max(tier.gain() * gate * stair))
}

/// Fill `out` with smooth tick values.
pub fn smooth_ladder_distance(
    distance: f64,
    ladder: &[Tier],
    params: SmoothParams,
    out: &mut [f64],
) -> Result<()> {
    validate_ladder(ladder)?;
    ensure_output(ladder.len(), out.len())?;
    for (slot, tier) in out.iter_mut().zip(ladder.iter().copied()) {
        *slot = smooth_tick_distance(distance, tier, params)?;
    }
    Ok(())
}

/// Allocate and return smooth tick values.
pub fn smooth_ladder_values(
    distance: f64,
    ladder: &[Tier],
    params: SmoothParams,
) -> Result<Vec<f64>> {
    let mut out = vec![0.0; ladder.len()];
    smooth_ladder_distance(distance, ladder, params, &mut out)?;
    Ok(out)
}

fn sigmoid(value: f64) -> f64 {
    let clipped = value.clamp(-60.0, 60.0);
    1.0 / (1.0 + (-clipped).exp())
}

fn smooth_stair(x: f64, sharpness: f64, max_stairs: usize) -> f64 {
    if x <= 0.0 {
        return sigmoid(sharpness * x);
    }
    let hard = x.ceil();
    if !hard.is_finite() || hard > max_stairs as f64 {
        return hard;
    }
    let j_max = hard as usize + 1;
    1.0 + (1..=j_max)
        .map(|j| sigmoid(sharpness * (x - j as f64)))
        .sum::<f64>()
}
