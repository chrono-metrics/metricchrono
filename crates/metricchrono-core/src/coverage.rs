use crate::{Metric, MetricChronoError, Result, Tier};
use serde::{Deserialize, Serialize};

/// Per-tier streaming coverage meter: a greedy maximal epsilon-packing of the
/// visited image.
///
/// The meter answers the question the per-step ladder cannot: "how much
/// *distinct* territory has this stream actually visited?"  Consecutive-pair
/// ticks (throughput) are silent below epsilon by design, so a stream can
/// relocate arbitrarily far through individually sub-threshold steps (creep)
/// or fire forever while bouncing between two states (churn).  Coverage is the
/// complementary read-out: it grows exactly when a sample lands at least
/// `epsilon_k` away from every stored representative at tier `k`, is invariant
/// under revisits and dwell, and stores representatives only -- never the
/// stream.  By the coverage bound, the representative count at tier `k` never
/// exceeds `1 + path_length / epsilon_k`.
///
/// The greedy count is sandwiched between the true packing numbers at
/// resolutions `epsilon` and `2 * epsilon` (a maximal separated set is
/// simultaneously a packing and a cover): a factor-two approximation in scale,
/// at most one tier of blur on a geometric ladder with `alpha >= 2`.
///
/// Distances that are NaN reject admission (conservative: a sample with an
/// undefined distance never becomes a representative).
#[derive(Clone, Debug)]
pub struct CoverageMeter<T> {
    epsilons: Vec<f64>,
    representatives: Vec<Vec<T>>,
}

impl<T: Clone> CoverageMeter<T> {
    /// Build a meter with one store per tier, using each tier's epsilon as its
    /// packing resolution.  Accepts a [`crate::Ladder`], a `Vec<Tier>`, or a
    /// tier slice.
    pub fn from_ladder(ladder: impl AsRef<[Tier]>) -> Self {
        let epsilons: Vec<f64> = ladder.as_ref().iter().map(|tier| tier.epsilon).collect();
        let representatives = vec![Vec::new(); epsilons.len()];
        Self {
            epsilons,
            representatives,
        }
    }

    /// Build a meter from explicit per-tier resolutions.
    pub fn from_epsilons(epsilons: impl Into<Vec<f64>>) -> Result<Self> {
        let epsilons = epsilons.into();
        if epsilons.is_empty() {
            return Err(MetricChronoError::EmptyLadder);
        }
        if epsilons
            .iter()
            .any(|epsilon| !epsilon.is_finite() || *epsilon <= 0.0)
        {
            return Err(MetricChronoError::InvalidArgument(
                "coverage epsilons must be finite and positive",
            ));
        }
        let representatives = vec![Vec::new(); epsilons.len()];
        Ok(Self {
            epsilons,
            representatives,
        })
    }

    /// Observe one sample.  Returns, per tier, whether the sample was admitted
    /// as a new representative (i.e. whether coverage grew at that tier).
    pub fn observe<M: Metric<T>>(&mut self, metric: &M, state: &T) -> Vec<bool> {
        let mut admitted = Vec::with_capacity(self.epsilons.len());
        for (tier, epsilon) in self.epsilons.iter().enumerate() {
            let separated = self.representatives[tier]
                .iter()
                .all(|representative| metric.distance(representative, state) >= *epsilon);
            if separated {
                self.representatives[tier].push(state.clone());
            }
            admitted.push(separated);
        }
        admitted
    }

    /// Number of tiers.
    pub fn tier_count(&self) -> usize {
        self.epsilons.len()
    }

    /// Per-tier packing resolutions.
    pub fn epsilons(&self) -> &[f64] {
        &self.epsilons
    }

    /// Coverage count at one tier.
    pub fn count(&self, tier: usize) -> Option<usize> {
        self.representatives.get(tier).map(Vec::len)
    }

    /// Coverage counts at every tier.
    pub fn counts(&self) -> Vec<usize> {
        self.representatives.iter().map(Vec::len).collect()
    }

    /// Stored representatives at one tier.
    pub fn representatives(&self, tier: usize) -> Option<&[T]> {
        self.representatives.get(tier).map(Vec::as_slice)
    }
}

/// Progress efficiency `(coverage - 1) * epsilon / path_length`: the fraction
/// of traversed metric length that acquired new epsilon-distinct territory.
/// Guaranteed in `[0, 1]` by the coverage bound.  Returns `None` when
/// `path_length` is not positive and finite.
pub fn progress_efficiency(coverage: usize, epsilon: f64, path_length: f64) -> Option<f64> {
    if !path_length.is_finite() || path_length <= 0.0 {
        return None;
    }
    let gained = coverage.saturating_sub(1) as f64 * epsilon;
    Some((gained / path_length).clamp(0.0, 1.0))
}

/// Joint operating regime of a window, from the throughput increment (path sum
/// of ticks) and the coverage increment (newly admitted representatives).
///
/// Throughput and coverage are independent axes: neither bounds the other.
/// Their joint signs classify the window:
///
/// - `Quiescent`: neither grew -- nothing salient happened.
/// - `Progress`: both grew -- salient change acquiring new territory.
/// - `Churn`: throughput without coverage -- supra-threshold motion among
///   already-visited states (thrash, oscillation, retry storms, livelock).
/// - `Creep`: coverage without throughput -- relocation through individually
///   sub-threshold steps (gradual degradation, sensor bias walk, an adversary
///   below the detection floor).  This is the structural blind spot of
///   per-step thresholding; coverage is its audit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OperatingRegime {
    Quiescent,
    Progress,
    Churn,
    Creep,
}

/// Classify a window from its throughput and coverage increments.
pub fn classify_regime(throughput_delta: f64, coverage_delta: usize) -> OperatingRegime {
    let ticked = throughput_delta > 0.0;
    let covered = coverage_delta > 0;
    match (ticked, covered) {
        (false, false) => OperatingRegime::Quiescent,
        (true, true) => OperatingRegime::Progress,
        (true, false) => OperatingRegime::Churn,
        (false, true) => OperatingRegime::Creep,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Absolute, Ladder, MetricFn};

    fn meter_eps(epsilons: &[f64]) -> CoverageMeter<f64> {
        CoverageMeter::from_epsilons(epsilons.to_vec()).expect("valid epsilons")
    }

    #[test]
    fn creep_grows_coverage_with_zero_throughput() {
        let ladder = Ladder::geometric(0.1, 0.15, 2.0, 4, 0.0, 1.0).expect("ladder");
        let mut meter = CoverageMeter::from_ladder(&ladder);
        let metric = Absolute;
        let mut throughput = 0.0;
        let mut position = 0.0_f64;
        meter.observe(&metric, &position);
        for _ in 0..100 {
            let next = position + 0.05; // below every tier epsilon
            throughput += ladder
                .values((next - position).abs())
                .expect("tick")
                .iter()
                .sum::<f64>();
            meter.observe(&metric, &next);
            position = next;
        }
        assert_eq!(throughput, 0.0, "creep steps must be silent at every tier");
        let coverage = meter.count(0).expect("tier 0");
        // ideal spacing admits ~51 representatives over a path of 5.0; float
        // accumulation pushes nominal 0.10 separations just below epsilon, so
        // admissions land every ~0.13 -- the claim under test is only that
        // coverage registers relocation (far above the churn ceiling of 2)
        assert!(
            coverage > 30,
            "coverage must register sub-threshold relocation, got {coverage}"
        );
        assert_eq!(
            classify_regime(throughput, coverage - 1),
            OperatingRegime::Creep
        );
    }

    #[test]
    fn churn_freezes_coverage_while_throughput_grows() {
        let ladder = Ladder::geometric(0.1, 0.15, 2.0, 4, 0.0, 1.0).expect("ladder");
        let mut meter = CoverageMeter::from_ladder(&ladder);
        let metric = Absolute;
        let mut throughput = 0.0;
        let mut previous = 0.0_f64;
        meter.observe(&metric, &previous);
        for step in 0..200 {
            let next = if step % 2 == 0 { 0.5 } else { 0.0 };
            throughput += ladder
                .values((next - previous).abs())
                .expect("tick")
                .iter()
                .sum::<f64>();
            meter.observe(&metric, &next);
            previous = next;
        }
        assert!(throughput > 100.0);
        assert_eq!(meter.count(0), Some(2));
        assert_eq!(classify_regime(throughput, 0), OperatingRegime::Churn);
    }

    #[test]
    fn coverage_bound_holds_on_a_random_walk() {
        // deterministic LCG so the test needs no rng dependency
        let mut seed = 0x2545F4914F6CDD1D_u64;
        let mut uniform = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 11) as f64 / (1_u64 << 53) as f64
        };
        let epsilon = 0.1;
        let mut meter = meter_eps(&[epsilon]);
        let metric = MetricFn(|a: &f64, b: &f64| (a - b).abs());
        let mut path_length = 0.0;
        let mut position = 0.0_f64;
        meter.observe(&metric, &position);
        for _ in 0..500 {
            let step = (uniform() - 0.5) * 0.6;
            path_length += step.abs();
            position += step;
            meter.observe(&metric, &position);
        }
        let coverage = meter.count(0).expect("tier 0");
        assert!(
            path_length >= (coverage.saturating_sub(1)) as f64 * epsilon,
            "coverage bound violated: ell={path_length}, M={coverage}"
        );
        let efficiency =
            progress_efficiency(coverage, epsilon, path_length).expect("positive path");
        assert!((0.0..=1.0).contains(&efficiency));
    }

    #[test]
    fn quadrants_are_exhaustive() {
        assert_eq!(classify_regime(0.0, 0), OperatingRegime::Quiescent);
        assert_eq!(classify_regime(3.0, 2), OperatingRegime::Progress);
        assert_eq!(classify_regime(3.0, 0), OperatingRegime::Churn);
        assert_eq!(classify_regime(0.0, 5), OperatingRegime::Creep);
    }

    #[test]
    fn rejects_invalid_epsilons() {
        assert!(CoverageMeter::<f64>::from_epsilons(Vec::<f64>::new()).is_err());
        assert!(CoverageMeter::<f64>::from_epsilons(vec![0.0]).is_err());
        assert!(CoverageMeter::<f64>::from_epsilons(vec![f64::NAN]).is_err());
    }

    #[test]
    fn nan_distances_reject_admission() {
        let mut meter = meter_eps(&[0.1]);
        let metric = MetricFn(|_: &f64, _: &f64| f64::NAN);
        meter.observe(&metric, &0.0);
        meter.observe(&metric, &10.0);
        assert_eq!(meter.count(0), Some(1));
    }
}
