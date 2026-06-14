use crate::{
    classify_regime, CfarDecision, CoverageMeter, EventId, EventLog, GuardedQuantileThreshold,
    Ladder, Metric, OperatingRegime, Result,
};

/// Result of a single [`Session::observe`] call.
#[derive(Clone, Debug)]
pub struct StepResult {
    /// Tick vector across all tiers for the distance from the previous state.
    pub ticks: Vec<f64>,
    /// Raw metric distance from the previous state (0.0 on the first step).
    pub distance: f64,
    /// Position of this record in the session's [`EventLog`].
    pub event_id: EventId,
    /// Per-tier coverage admission: `true` at tier `k` when this state became a
    /// new representative, meaning coverage grew at that scale.
    pub admitted: Vec<bool>,
    /// CFAR decision on the raw distance, if a threshold was configured and has
    /// warmed up.
    pub cfar: Option<CfarDecision>,
    /// Per-step operating regime derived from this step's throughput and coverage.
    pub regime: OperatingRegime,
    /// 1-indexed step counter.
    pub step: u64,
}

/// Pipeline that wires [`Ladder`] + [`EventLog`] + [`CoverageMeter`] +
/// optional [`GuardedQuantileThreshold`] into a single `observe` call.
///
/// Without `Session`, the consumer must manually instantiate and feed the same
/// observation to four separate objects, tracking tier counts across all of
/// them.  `Session` eliminates that glue.
pub struct Session<T> {
    ladder: Ladder,
    event_log: EventLog<u64>,
    coverage: CoverageMeter<T>,
    cfar: Option<GuardedQuantileThreshold>,
    previous: Option<T>,
    tick_buffer: Vec<f64>,
    admitted_buffer: Vec<bool>,
    step_count: u64,
}

impl<T: Clone> Session<T> {
    /// Create a session with ladder-derived event log and coverage meter.
    pub fn new(ladder: Ladder) -> Self {
        let tier_count = ladder.len();
        let event_log = EventLog::new(tier_count).expect("Ladder is non-empty");
        let coverage = CoverageMeter::from_ladder(&ladder);
        Self {
            ladder,
            event_log,
            coverage,
            cfar: None,
            previous: None,
            tick_buffer: vec![0.0; tier_count],
            admitted_buffer: vec![false; tier_count],
            step_count: 0,
        }
    }

    /// Create a session with a guarded CFAR anomaly threshold on the raw
    /// inter-state distance.
    pub fn with_cfar(
        ladder: Ladder,
        reference_len: usize,
        guard_len: usize,
        target_fp: f64,
    ) -> Result<Self> {
        let cfar = GuardedQuantileThreshold::new(reference_len, guard_len, target_fp)?;
        let mut session = Self::new(ladder);
        session.cfar = Some(cfar);
        Ok(session)
    }

    /// Observe one state.  Computes the distance from the previous state (0.0
    /// on the first call), feeds it through the ladder, records the tick vector
    /// in the event log, updates coverage, runs the CFAR (if configured), and
    /// classifies the operating regime.
    pub fn observe<M: Metric<T>>(&mut self, metric: &M, state: &T) -> StepResult {
        self.step_count += 1;

        let distance = self
            .previous
            .as_ref()
            .map(|prev| metric.distance(prev, state))
            .unwrap_or(0.0);

        self.ladder
            .distance_into(distance, &mut self.tick_buffer)
            .expect("buffer sized to ladder");

        let event_id = self
            .event_log
            .append(self.step_count, self.tick_buffer.clone())
            .expect("tick vector matches tier count");

        self.coverage
            .observe_into(metric, state, &mut self.admitted_buffer)
            .expect("admitted buffer sized to tier count");

        let cfar_decision = self.cfar.as_mut().and_then(|cfar| cfar.observe(distance));

        let throughput: f64 = self.tick_buffer.iter().sum();
        let coverage_delta = self.admitted_buffer.iter().filter(|&&a| a).count();
        let regime = classify_regime(throughput, coverage_delta);

        self.previous = Some(state.clone());

        StepResult {
            ticks: self.tick_buffer.clone(),
            distance,
            event_id,
            admitted: self.admitted_buffer.clone(),
            cfar: cfar_decision,
            regime,
            step: self.step_count,
        }
    }

    pub fn ladder(&self) -> &Ladder {
        &self.ladder
    }

    pub fn event_log(&self) -> &EventLog<u64> {
        &self.event_log
    }

    pub fn coverage(&self) -> &CoverageMeter<T> {
        &self.coverage
    }

    pub fn coverage_mut(&mut self) -> &mut CoverageMeter<T> {
        &mut self.coverage
    }

    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    pub fn tier_count(&self) -> usize {
        self.ladder.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Absolute;

    fn test_ladder() -> Ladder {
        Ladder::geometric(0.1, 0.15, 2.0, 4, 0.0, 1.0).expect("valid ladder")
    }

    #[test]
    fn first_step_has_zero_distance_and_admits_everywhere() {
        let mut session = Session::new(test_ladder());
        let result = session.observe(&Absolute, &0.0);
        assert_eq!(result.step, 1);
        assert_eq!(result.distance, 0.0);
        assert!(result.ticks.iter().all(|&t| t == 0.0));
        assert!(result.admitted.iter().all(|&a| a));
        assert_eq!(result.regime, OperatingRegime::Creep);
    }

    #[test]
    fn large_step_produces_ticks_and_progress() {
        let mut session = Session::new(test_ladder());
        session.observe(&Absolute, &0.0);
        let result = session.observe(&Absolute, &5.0);
        assert_eq!(result.distance, 5.0);
        assert!(result.ticks.iter().any(|&t| t > 0.0));
        assert!(result.admitted.iter().any(|&a| a));
        assert_eq!(result.regime, OperatingRegime::Progress);
    }

    #[test]
    fn churn_detected_on_oscillation() {
        let mut session = Session::new(test_ladder());
        session.observe(&Absolute, &0.0);
        session.observe(&Absolute, &5.0);

        // Now oscillate between two known states — coverage won't grow
        for i in 0..10 {
            let state = if i % 2 == 0 { 0.0 } else { 5.0 };
            let result = session.observe(&Absolute, &state);
            assert!(result.ticks.iter().any(|&t| t > 0.0));
            assert_eq!(result.regime, OperatingRegime::Churn);
        }
    }

    #[test]
    fn event_log_grows_with_steps() {
        let mut session = Session::new(test_ladder());
        for i in 0..5 {
            let result = session.observe(&Absolute, &(i as f64));
            assert_eq!(result.event_id, i);
        }
        assert_eq!(session.event_log().len(), 5);
        assert_eq!(session.step_count(), 5);
    }

    #[test]
    fn cfar_warms_up_then_decides() {
        let mut session = Session::with_cfar(test_ladder(), 5, 2, 0.2).expect("valid cfar config");
        // First 7 steps (5 ref + 2 guard): CFAR returns None
        for i in 0..8 {
            let result = session.observe(&Absolute, &(i as f64 * 0.01));
            if i < 7 {
                assert!(result.cfar.is_none(), "step {i} should be warming up");
            }
        }
        // Step 8: CFAR should produce a decision
        let result = session.observe(&Absolute, &100.0);
        let decision = result.cfar.expect("CFAR should be warm");
        assert!(decision.alarm, "large jump should alarm");
    }

    #[test]
    fn coverage_counts_accessible_through_session() {
        let mut session = Session::new(test_ladder());
        for i in 0..10 {
            session.observe(&Absolute, &(i as f64 * 2.0));
        }
        let counts = session.coverage().counts();
        assert_eq!(counts.len(), 4);
        assert!(counts[0] >= counts[3], "finer tiers admit more");
    }

    #[test]
    fn cumulative_sub_threshold_drift_produces_creep() {
        let mut session = Session::new(test_ladder());
        session.observe(&Absolute, &0.0);
        // Each step of 0.05 is below epsilon_0 = 0.1, so ticks are always zero.
        // But after enough steps the position drifts past epsilon from the
        // initial representative, producing coverage growth without throughput.
        let mut saw_creep = false;
        for i in 1..=20 {
            let result = session.observe(&Absolute, &(i as f64 * 0.05));
            assert!(result.ticks.iter().all(|&t| t == 0.0));
            if result.regime == OperatingRegime::Creep {
                saw_creep = true;
            }
        }
        assert!(saw_creep, "drift past epsilon should register as Creep");
    }

    #[test]
    fn quiescent_on_revisit_below_epsilon() {
        let mut session = Session::new(test_ladder());
        session.observe(&Absolute, &0.0);
        // Revisit the same state — zero distance, no new coverage
        let result = session.observe(&Absolute, &0.0);
        assert_eq!(result.distance, 0.0);
        assert!(result.admitted.iter().all(|&a| !a));
        assert_eq!(result.regime, OperatingRegime::Quiescent);
    }
}
