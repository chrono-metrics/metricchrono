use crate::{MetricChronoError, Result, Tier};

const MAX_PROMOTION_DEPTH: usize = 1000;

/// Tick normalization modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Normalization {
    None,
    UnitMax,
    Tanh,
}

/// Owned validated ladder configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct Ladder {
    tiers: Vec<Tier>,
}

/// Owned tick-vector output.
pub type TickVector = Vec<f64>;

impl Ladder {
    pub fn new(tiers: impl Into<Vec<Tier>>) -> Result<Self> {
        let tiers = custom_ladder(tiers)?;
        Ok(Self { tiers })
    }

    pub fn geometric(
        epsilon0: f64,
        delta0: f64,
        ratio: f64,
        tiers: usize,
        p: f64,
        epsilon_ref: f64,
    ) -> Result<Self> {
        Ok(Self {
            tiers: geometric_ladder(epsilon0, delta0, ratio, tiers, p, epsilon_ref)?,
        })
    }

    pub fn len(&self) -> usize {
        self.tiers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiers.is_empty()
    }

    pub fn tiers(&self) -> &[Tier] {
        &self.tiers
    }

    pub fn distance_into(&self, distance: f64, out: &mut [f64]) -> Result<()> {
        ladder_distance(distance, &self.tiers, out)
    }

    pub fn values(&self, distance: f64) -> Result<TickVector> {
        ladder_values(distance, &self.tiers)
    }
}

impl AsRef<[Tier]> for Ladder {
    fn as_ref(&self) -> &[Tier] {
        self.tiers()
    }
}

/// Compute the single-scale epsilon-delta-p tick.
///
/// This function assumes `tier` is valid. Use [`try_tick_distance`] when inputs
/// may be user-provided.
pub fn tick_distance(distance: f64, tier: Tier) -> f64 {
    let d = sanitize_distance(distance);
    if d < tier.epsilon {
        0.0
    } else {
        finite_or_max(tier.gain() * (d / tier.delta).ceil())
    }
}

/// Validate the tier and compute a single-scale tick.
pub fn try_tick_distance(distance: f64, tier: Tier) -> Result<f64> {
    tier.validate_at(0)?;
    ensure_distance(distance)?;
    Ok(tick_distance(distance, tier))
}

/// Fill `out` with the tick vector for `distance` across `ladder`.
pub fn ladder_distance(distance: f64, ladder: &[Tier], out: &mut [f64]) -> Result<()> {
    validate_ladder(ladder)?;
    ensure_distance(distance)?;
    ensure_output(ladder.len(), out.len())?;
    for (slot, tier) in out.iter_mut().zip(ladder.iter().copied()) {
        *slot = tick_distance(distance, tier);
    }
    Ok(())
}

/// Allocate and return the tick vector for `distance`.
pub fn ladder_values(distance: f64, ladder: &[Tier]) -> Result<Vec<f64>> {
    let mut out = vec![0.0; ladder.len()];
    ladder_distance(distance, ladder, &mut out)?;
    Ok(out)
}

/// Construct a geometric ladder with matching epsilon and delta ratios.
pub fn geometric_ladder(
    epsilon0: f64,
    delta0: f64,
    ratio: f64,
    tiers: usize,
    p: f64,
    epsilon_ref: f64,
) -> Result<Vec<Tier>> {
    if tiers == 0 {
        return Err(MetricChronoError::EmptyLadder);
    }
    if !ratio.is_finite() || ratio <= 1.0 {
        return Err(MetricChronoError::InvalidArgument(
            "ratio must be finite and > 1",
        ));
    }
    let mut ladder = Vec::with_capacity(tiers);
    for k in 0..tiers {
        let scale = ratio.powi(k as i32);
        ladder.push(Tier::new(epsilon0 * scale, delta0 * scale, p, epsilon_ref)?);
    }
    validate_ladder(&ladder)?;
    Ok(ladder)
}

/// Validate and return a custom ladder.
pub fn custom_ladder(tiers: impl Into<Vec<Tier>>) -> Result<Vec<Tier>> {
    let ladder = tiers.into();
    validate_ladder(&ladder)?;
    Ok(ladder)
}

/// Validate ladder shape and monotonic epsilon ordering.
pub fn validate_ladder(ladder: &[Tier]) -> Result<()> {
    if ladder.is_empty() {
        return Err(MetricChronoError::EmptyLadder);
    }
    for (index, tier) in ladder.iter().copied().enumerate() {
        tier.validate_at(index)?;
        if index > 0 && tier.epsilon <= ladder[index - 1].epsilon {
            return Err(MetricChronoError::InvalidTier {
                index,
                reason: "epsilon values must be strictly increasing",
            });
        }
        if index > 0 && tier.delta <= ladder[index - 1].delta {
            return Err(MetricChronoError::InvalidTier {
                index,
                reason: "delta values must be strictly increasing",
            });
        }
    }
    Ok(())
}

/// Normalize a tick vector into `out`.
pub fn normalize_ticks(input: &[f64], mode: Normalization, out: &mut [f64]) -> Result<()> {
    ensure_output(input.len(), out.len())?;
    match mode {
        Normalization::None => out[..input.len()].copy_from_slice(input),
        Normalization::UnitMax => {
            let max_abs = input
                .iter()
                .copied()
                .filter(|value| value.is_finite())
                .map(f64::abs)
                .fold(0.0, f64::max);
            if max_abs <= 0.0 {
                out[..input.len()].fill(0.0);
            } else {
                for (slot, value) in out.iter_mut().zip(input.iter().copied()) {
                    *slot = sanitize_signed(value) / max_abs;
                }
            }
        }
        Normalization::Tanh => {
            for (slot, value) in out.iter_mut().zip(input.iter().copied()) {
                *slot = sanitize_signed(value).tanh();
            }
        }
    }
    Ok(())
}

/// Default carry quotas derived from ladder epsilons.
pub fn carry_rules(epsilons: &[f64]) -> Result<Vec<u64>> {
    if epsilons.is_empty() {
        return Err(MetricChronoError::EmptyLadder);
    }
    epsilons
        .iter()
        .enumerate()
        .map(|(index, epsilon)| {
            if !epsilon.is_finite() || *epsilon <= 0.0 {
                return Err(MetricChronoError::InvalidTier {
                    index,
                    reason: "epsilon must be finite and > 0",
                });
            }
            Ok(epsilon.ceil().max(1.0) as u64)
        })
        .collect()
}

/// Basic promotion/carry counter for ladder events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionCounter {
    quotas: Vec<u64>,
    counters: Vec<u64>,
}

impl PromotionCounter {
    pub fn new(quotas: impl Into<Vec<u64>>) -> Result<Self> {
        let quotas = quotas.into();
        if quotas.is_empty() {
            return Err(MetricChronoError::EmptyLadder);
        }
        if quotas.contains(&0) {
            return Err(MetricChronoError::InvalidArgument(
                "promotion quotas must be > 0",
            ));
        }
        let counters = vec![0; quotas.len()];
        Ok(Self { quotas, counters })
    }

    pub fn from_epsilons(epsilons: &[f64]) -> Result<Self> {
        Self::new(carry_rules(epsilons)?)
    }

    pub fn len(&self) -> usize {
        self.quotas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.quotas.is_empty()
    }

    pub fn quotas(&self) -> &[u64] {
        &self.quotas
    }

    pub fn counters(&self) -> &[u64] {
        &self.counters
    }

    pub fn reset(&mut self) {
        self.counters.fill(0);
    }

    /// Advance counters and write promotion flags into `out`.
    pub fn step(&mut self, event_flags: Option<&[bool]>, out: &mut [bool]) -> Result<()> {
        let len = self.quotas.len();
        ensure_output(len, out.len())?;
        if let Some(flags) = event_flags {
            ensure_shape(len, flags.len(), "event flags")?;
        }

        out[..len].fill(false);
        for (k, counter) in self.counters.iter_mut().enumerate().take(len) {
            let event = event_flags.map(|flags| flags[k]).unwrap_or(false);
            if !event {
                *counter = counter.saturating_add(1);
            }
        }

        let mut depth = 0;
        loop {
            let mut changed = false;
            for (k, promoted) in out.iter_mut().enumerate().take(len) {
                if self.counters[k] < self.quotas[k] {
                    continue;
                }
                self.counters[k] = 0;
                *promoted = true;
                if k + 1 < len {
                    self.counters[k + 1] = self.counters[k + 1].saturating_add(1);
                }
                changed = true;
            }
            if !changed {
                break;
            }
            depth += 1;
            if depth > MAX_PROMOTION_DEPTH {
                return Err(MetricChronoError::InvalidArgument(
                    "promotion depth exceeded",
                ));
            }
        }

        if let Some(flags) = event_flags {
            for (k, event) in flags.iter().copied().enumerate().take(len) {
                if event {
                    self.counters[k] = 0;
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn ensure_output(needed: usize, actual: usize) -> Result<()> {
    if actual < needed {
        Err(MetricChronoError::OutputTooSmall { needed, actual })
    } else {
        Ok(())
    }
}

pub(crate) fn ensure_shape(expected: usize, actual: usize, context: &'static str) -> Result<()> {
    if expected != actual {
        Err(MetricChronoError::ShapeMismatch {
            expected,
            actual,
            context,
        })
    } else {
        Ok(())
    }
}

pub(crate) fn ensure_distance(distance: f64) -> Result<()> {
    if !distance.is_finite() || distance < 0.0 {
        Err(MetricChronoError::InvalidArgument(
            "distance must be finite and >= 0",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn sanitize_distance(distance: f64) -> f64 {
    if distance.is_nan() || distance.is_sign_negative() {
        0.0
    } else if distance.is_infinite() {
        f64::MAX
    } else {
        distance
    }
}

pub(crate) fn sanitize_signed(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else if value == f64::INFINITY {
        f64::MAX
    } else if value == f64::NEG_INFINITY {
        -f64::MAX
    } else {
        value
    }
}

pub(crate) fn finite_or_max(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else if value.is_infinite() {
        f64::MAX
    } else {
        value
    }
}
