use std::cell::RefCell;

use crate::{MetricChronoError, Result};

/// CFAR threshold decision for one scalar statistic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CfarDecision {
    /// Guarded order-statistic threshold computed from the reference window.
    pub threshold: f64,
    /// Whether the observed statistic strictly exceeds `threshold`.
    pub alarm: bool,
}

/// Causal guarded order-statistic CFAR threshold over a scalar statistic stream.
///
/// At each non-NaN test value, the prior non-NaN history is laid out as
/// `[R reference values][G guard values][test value]`.  The test value is never
/// part of its own reference set: the threshold and strict `value > threshold`
/// alarm are computed before the finite value is pushed into the ring.
///
/// The 1-indexed order-statistic rank is
/// `ceil((1 - target_fp) * (R + 1))`, clamped to `[1, R]`.  When
/// `target_fp >= 1 / (R + 1)`, a tie-free exchangeable test value has uniform
/// rank among the `R + 1` values, so
/// `P(value > threshold) = (R + 1 - rank) / (R + 1) <= target_fp`.  Smaller
/// requested targets clamp to rank `R`, the most conservative available
/// threshold; the achievable tie-free rate is then the finite-reference floor
/// `1 / (R + 1)`, which is above the requested target.  With ties, the strict
/// `>` rule is conservative.
///
/// NaN is the safe no-decision input, mirroring the coverage meter's
/// NaN-rejecting behavior: NaN observations return `None`, never alarm, and are
/// never admitted to the reference/guard history.
#[derive(Clone, Debug)]
pub struct GuardedQuantileThreshold {
    reference_len: usize,
    guard_len: usize,
    target_fp: f64,
    rank: usize,
    order_index: usize,
    history: Ring,
    scratch: RefCell<Vec<f64>>,
}

impl GuardedQuantileThreshold {
    /// Build a guarded CFAR threshold estimator.
    pub fn new(reference_len: usize, guard_len: usize, target_fp: f64) -> Result<Self> {
        validate_guarded_config(reference_len, target_fp)?;
        let rank = cfar_rank(reference_len, target_fp);
        Ok(Self {
            reference_len,
            guard_len,
            target_fp,
            rank,
            order_index: rank - 1,
            history: Ring::new(reference_len + guard_len),
            scratch: RefCell::new(Vec::with_capacity(reference_len)),
        })
    }

    /// Number of guarded reference values, `R`.
    pub fn reference_len(&self) -> usize {
        self.reference_len
    }

    /// Number of guard values, `G`.
    pub fn guard_len(&self) -> usize {
        self.guard_len
    }

    /// Target upper-tail false-positive probability.
    pub fn target_fp(&self) -> f64 {
        self.target_fp
    }

    /// 1-indexed order-statistic rank used for the reference threshold.
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// Number of non-NaN prior observations currently retained.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Whether no non-NaN prior observations are currently retained.
    pub fn is_empty(&self) -> bool {
        self.history.len() == 0
    }

    /// Whether fewer than `R + G` non-NaN prior observations are available.
    pub fn is_warming_up(&self) -> bool {
        self.history.len() < self.history.capacity()
    }

    /// Observe one statistic and, once warm, return the causal CFAR decision.
    ///
    /// Returns `None` until `R + G` non-NaN prior values are available.  A NaN
    /// statistic is a safe no-decision input: it returns `None` and is not
    /// pushed into the ring.
    pub fn observe(&mut self, value: f64) -> Option<CfarDecision> {
        if value.is_nan() {
            return None;
        }
        let threshold = self.current_threshold();
        let decision = threshold.map(|threshold| CfarDecision {
            threshold,
            alarm: value > threshold,
        });
        self.history.push(value);
        decision
    }

    /// Current guarded threshold for the next non-NaN observation.
    pub fn threshold(&self) -> Option<f64> {
        self.current_threshold()
    }

    /// Self-reported smallest visible event at gain `kappa`.
    ///
    /// Returns `kappa * threshold` for the current guarded threshold, or `None`
    /// while warming up.  Non-finite `kappa` has no meaningful detection floor
    /// and returns `None`.
    pub fn detection_floor(&self, kappa: f64) -> Option<f64> {
        if !kappa.is_finite() {
            return None;
        }
        self.current_threshold().map(|threshold| kappa * threshold)
    }

    fn current_threshold(&self) -> Option<f64> {
        if self.is_warming_up() {
            return None;
        }
        let mut scratch = self.scratch.borrow_mut();
        scratch.clear();
        for index in 0..self.reference_len {
            scratch.push(self.history.get(index));
        }
        let (_, threshold, _) = scratch.select_nth_unstable_by(self.order_index, f64::total_cmp);
        Some(*threshold)
    }
}

/// Causal guarded robust scale estimator for distance normalization.
///
/// The retained history uses the same `[R reference][G guard][test]` layout as
/// [`GuardedQuantileThreshold`].  The scale is the median of the `R` reference
/// distances, clamped below by `scale_floor`.  The floor is intentional: an
/// instrument's true resolution bounds the effective gain, so normalization
/// must never divide by a scale below the measurement floor.
///
/// NaN distances are safe no-decision inputs: they are not admitted to the
/// history and do not produce a scale observation.
#[derive(Clone, Debug)]
pub struct GuardedAmbientScale {
    reference_len: usize,
    guard_len: usize,
    scale_floor: f64,
    history: Ring,
    scratch: RefCell<Vec<f64>>,
}

impl GuardedAmbientScale {
    /// Build a guarded median scale estimator.
    pub fn new(reference_len: usize, guard_len: usize, scale_floor: f64) -> Result<Self> {
        if reference_len == 0 {
            return Err(MetricChronoError::InvalidArgument(
                "reference_len must be > 0",
            ));
        }
        if !scale_floor.is_finite() || scale_floor <= 0.0 {
            return Err(MetricChronoError::InvalidArgument(
                "scale_floor must be finite and > 0",
            ));
        }
        Ok(Self {
            reference_len,
            guard_len,
            scale_floor,
            history: Ring::new(reference_len + guard_len),
            scratch: RefCell::new(Vec::with_capacity(reference_len)),
        })
    }

    /// Number of guarded reference distances, `R`.
    pub fn reference_len(&self) -> usize {
        self.reference_len
    }

    /// Number of guard distances, `G`.
    pub fn guard_len(&self) -> usize {
        self.guard_len
    }

    /// Lower bound applied to the median reference scale.
    pub fn scale_floor(&self) -> f64 {
        self.scale_floor
    }

    /// Number of non-NaN prior distances currently retained.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Whether no non-NaN prior distances are currently retained.
    pub fn is_empty(&self) -> bool {
        self.history.len() == 0
    }

    /// Whether fewer than `R + G` non-NaN prior distances are available.
    pub fn is_warming_up(&self) -> bool {
        self.history.len() < self.history.capacity()
    }

    /// Observe one distance, returning the causal scale before it is pushed.
    ///
    /// Returns `None` while warming up.  A NaN distance is not admitted and
    /// returns `None`.
    pub fn observe(&mut self, distance: f64) -> Option<f64> {
        if distance.is_nan() {
            return None;
        }
        let scale = self.scale();
        self.history.push(distance);
        scale
    }

    /// Current guarded, floored median scale for the next non-NaN distance.
    pub fn scale(&self) -> Option<f64> {
        if self.is_warming_up() {
            return None;
        }
        let mut scratch = self.scratch.borrow_mut();
        scratch.clear();
        for index in 0..self.reference_len {
            scratch.push(self.history.get(index));
        }
        scratch.sort_by(f64::total_cmp);
        let mid = self.reference_len / 2;
        let median = if self.reference_len % 2 == 1 {
            scratch[mid]
        } else {
            0.5 * (scratch[mid - 1] + scratch[mid])
        };
        Some(median.max(self.scale_floor))
    }

    /// Normalize `distance` by the current guarded scale.
    ///
    /// Returns `None` while warming up or when `distance` is NaN.  The divisor
    /// is the median reference scale clamped by [`Self::scale_floor`].
    pub fn normalize(&self, distance: f64) -> Option<f64> {
        if distance.is_nan() {
            return None;
        }
        self.scale().map(|scale| distance / scale)
    }
}

fn validate_guarded_config(reference_len: usize, target_fp: f64) -> Result<()> {
    if reference_len == 0 {
        return Err(MetricChronoError::InvalidArgument(
            "reference_len must be > 0",
        ));
    }
    if !target_fp.is_finite() || !(0.0..1.0).contains(&target_fp) {
        return Err(MetricChronoError::InvalidArgument(
            "target_fp must lie in (0, 1)",
        ));
    }
    Ok(())
}

fn cfar_rank(reference_len: usize, target_fp: f64) -> usize {
    let raw = ((1.0 - target_fp) * (reference_len + 1) as f64).ceil() as usize;
    raw.clamp(1, reference_len)
}

#[derive(Clone, Debug)]
struct Ring {
    values: Vec<f64>,
    start: usize,
    len: usize,
}

impl Ring {
    fn new(capacity: usize) -> Self {
        Self {
            values: vec![0.0; capacity],
            start: 0,
            len: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.values.len()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn get(&self, chronological_index: usize) -> f64 {
        debug_assert!(chronological_index < self.len);
        self.values[(self.start + chronological_index) % self.capacity()]
    }

    fn push(&mut self, value: f64) {
        debug_assert!(!value.is_nan());
        let capacity = self.capacity();
        if self.len < capacity {
            self.values[(self.start + self.len) % capacity] = value;
            self.len += 1;
        } else {
            self.values[self.start] = value;
            self.start = (self.start + 1) % capacity;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg {
        state: u64,
    }

    impl Lcg {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn unit(&mut self) -> f64 {
            self.state = self
                .state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.state >> 11) as f64 / (1_u64 << 53) as f64
        }
    }

    fn exact_fp(reference_len: usize, target_fp: f64) -> f64 {
        let rank = cfar_rank(reference_len, target_fp);
        (reference_len + 1 - rank) as f64 / (reference_len + 1) as f64
    }

    #[test]
    fn rank_uniformity_matches_exact_finite_sample_rate() {
        for (reference_len, target_fp, seed) in [
            (19, 0.20, 0x2545F4914F6CDD1D_u64),
            (31, 0.05, 0x9E3779B97F4A7C15_u64),
        ] {
            let guard_len = 3;
            let mut rng = Lcg::new(seed);
            let mut cfar =
                GuardedQuantileThreshold::new(reference_len, guard_len, target_fp).unwrap();
            let mut alarms = 0_usize;
            let mut decisions = 0_usize;
            for _ in 0..250_000 {
                if let Some(decision) = cfar.observe(rng.unit()) {
                    decisions += 1;
                    alarms += usize::from(decision.alarm);
                }
            }
            let expected = exact_fp(reference_len, target_fp);
            let observed = alarms as f64 / decisions as f64;
            let se = (expected * (1.0 - expected) / decisions as f64).sqrt();
            assert!(
                (observed - expected).abs() <= 5.0 * se,
                "R={reference_len} fp={target_fp}: observed {observed}, expected {expected}, se {se}"
            );
        }
    }

    #[test]
    fn guard_excludes_recent_burst_from_threshold() {
        let mut guarded = GuardedQuantileThreshold::new(4, 2, 0.25).unwrap();
        for value in [1.0, 1.0, 1.0, 1.0, 100.0, 100.0] {
            assert!(guarded.observe(value).is_none());
        }
        let decision = guarded.observe(2.0).expect("warm decision");
        assert_eq!(decision.threshold, 1.0);
        assert!(decision.alarm);

        let mut unguarded_reference = GuardedQuantileThreshold::new(4, 2, 0.25).unwrap();
        for value in [1.0, 1.0, 100.0, 100.0, 1.0, 1.0] {
            assert!(unguarded_reference.observe(value).is_none());
        }
        let decision = unguarded_reference.observe(2.0).expect("warm decision");
        assert_eq!(decision.threshold, 100.0);
        assert!(!decision.alarm);
    }

    #[test]
    fn warmup_returns_none_exactly_reference_plus_guard_times() {
        let mut cfar = GuardedQuantileThreshold::new(3, 2, 0.2).unwrap();
        for index in 0..5 {
            assert!(cfar.observe(index as f64).is_none());
        }
        assert!(cfar.observe(10.0).is_some());
    }

    #[test]
    fn nan_is_not_admitted_and_never_decides() {
        let mut cfar = GuardedQuantileThreshold::new(2, 1, 0.5).unwrap();
        assert!(cfar.observe(1.0).is_none());
        assert!(cfar.observe(f64::NAN).is_none());
        assert_eq!(cfar.len(), 1);
        assert!(cfar.observe(2.0).is_none());
        assert!(cfar.observe(3.0).is_none());

        let before = cfar.threshold().expect("warm threshold");
        assert!(cfar.observe(f64::NAN).is_none());
        assert_eq!(cfar.len(), 3);
        assert_eq!(cfar.threshold(), Some(before));
        let decision = cfar.observe(4.0).expect("decision after skipped NaN");
        assert_eq!(decision.threshold, before);
    }

    #[test]
    fn scale_floor_clamps_normalization_gain() {
        let mut scale = GuardedAmbientScale::new(3, 0, 0.5).unwrap();
        assert!(scale.observe(0.1).is_none());
        assert!(scale.observe(0.2).is_none());
        assert!(scale.observe(0.1).is_none());
        assert_eq!(scale.scale(), Some(0.5));
        assert_eq!(scale.normalize(1.0), Some(2.0));
    }

    #[test]
    fn scale_uses_guarded_reference_median() {
        let mut scale = GuardedAmbientScale::new(4, 2, 0.01).unwrap();
        for value in [1.0, 2.0, 3.0, 100.0, 1000.0, 1000.0] {
            assert!(scale.observe(value).is_none());
        }
        assert_eq!(scale.scale(), Some(2.5));
        assert_eq!(scale.observe(10.0), Some(2.5));
    }

    #[test]
    fn decision_is_causal_before_test_value_is_pushed() {
        let mut cfar = GuardedQuantileThreshold::new(3, 0, 0.25).unwrap();
        for value in [1.0, 2.0, 3.0] {
            assert!(cfar.observe(value).is_none());
        }
        let decision = cfar.observe(100.0).expect("warm decision");
        assert_eq!(decision.threshold, 3.0);
        assert!(decision.alarm);

        let next = cfar.observe(4.0).expect("next decision");
        assert_eq!(next.threshold, 100.0);
        assert!(!next.alarm);
    }
}
