use metricchrono_core::{
    adaptive_ladder_distance, adaptive_zoom_window, carry_rules, coherence_residual,
    coherence_residuals, custom_ladder, geometric_ladder, ladder_distance, ladder_pair,
    ladder_values, normalize_ticks, simple_weight_update, smooth_ladder_values,
    smooth_tick_distance, tick_distance, tick_pair, try_tick_distance, validate_ladder,
    weighted_consensus, zoom_ladder_distance, Absolute, DiagonalMahalanobis, Euclidean, EventLog,
    JensenShannon, KullbackLeibler, Ladder, Metric, MetricChronoError, Normalization,
    PromotionCounter, SmoothParams, Tier, ZoomPolicy,
};

#[test]
fn kernel_and_ladder_match_epsilon_delta_p_contract() {
    let tier = Tier::new(0.5, 1.0, 0.5, 1.0).unwrap();
    assert_eq!(tick_distance(0.49, tier), 0.0);
    assert_eq!(tick_distance(0.5, tier), 0.5_f64.sqrt());
    assert_eq!(tick_distance(1.2, tier), 2.0_f64.sqrt());
    assert!(try_tick_distance(-1.0, tier).is_err());
    assert!(try_tick_distance(f64::NAN, tier).is_err());
    assert!(try_tick_distance(f64::INFINITY, tier).is_err());
    assert!(Tier::new(1.0, 1.0, 0.0, 1.0).is_err());
    assert!(Tier::new(0.5, 1.0, f64::NAN, 1.0).is_err());
    assert!(Tier::new(0.5, 1.0, 0.0, 0.0).is_err());
    assert!(Tier::builder().build().is_ok());

    let ladder = geometric_ladder(0.5, 1.0, 2.0, 3, 0.5, 1.0).unwrap();
    validate_ladder(&ladder).unwrap();
    let owned = Ladder::new(ladder.clone()).unwrap();
    assert_eq!(owned.len(), 3);
    assert_eq!(owned.tiers(), ladder.as_slice());
    let values = ladder_values(2.2, &ladder).unwrap();
    assert_eq!(values.len(), 3);
    assert!(values[0] > values[1]);
    assert!(values[2] > 0.0);

    let mut out = [0.0; 2];
    let err = ladder_distance(1.0, &ladder, &mut out).unwrap_err();
    assert_eq!(
        err,
        MetricChronoError::OutputTooSmall {
            needed: 3,
            actual: 2
        }
    );

    assert!(custom_ladder(vec![
        Tier::new(0.5, 1.0, 0.0, 1.0).unwrap(),
        Tier::new(0.75, 0.9, 0.0, 1.0).unwrap(),
    ])
    .is_err());

    let non_additive_tier = Tier::new(0.5, 1.0, 0.0, 1.0).unwrap();
    assert_ne!(
        tick_distance(0.4 + 0.4, non_additive_tier),
        tick_distance(0.4, non_additive_tier) + tick_distance(0.4, non_additive_tier)
    );
}

#[test]
fn public_metric_examples_include_divergence_and_mahalanobis() {
    assert_eq!(Absolute.distance(&2.0, &5.5), 3.5);

    let p = [0.2, 0.8];
    let q = [0.5, 0.5];

    let kl = KullbackLeibler::default().distance(&p, &q);
    let js = JensenShannon::default().distance(&p, &q);
    assert!(kl > 0.0);
    assert!(js > 0.0 && js < kl);

    let metric = DiagonalMahalanobis::from_variance([4.0, 1.0]);
    let distance = metric.distance(&[0.0, 0.0], &[4.0, 3.0]);
    assert!((distance - (13.0_f64).sqrt()).abs() < 1e-12);

    let tier = Tier::new(0.5, 1.0, 0.0, 1.0).unwrap();
    assert_eq!(tick_pair(&2.0, &3.5, &Absolute, tier).unwrap(), 2.0);
    assert!(ladder_pair(
        &[0.0, 0.0][..],
        &[1.0][..],
        &Euclidean,
        &[Tier::new(0.5, 1.0, 0.0, 1.0).unwrap()]
    )
    .is_err());
}

#[test]
fn custom_ladders_metrics_and_normalization_are_public_and_deterministic() {
    let ladder = custom_ladder(vec![
        Tier::new(0.5, 1.0, 0.0, 1.0).unwrap(),
        Tier::new(1.0, 2.0, 0.0, 1.0).unwrap(),
        Tier::new(2.0, 4.0, 0.0, 1.0).unwrap(),
    ])
    .unwrap();
    let metric = Euclidean;
    let values = ladder_pair(&[0.0, 0.0][..], &[3.0, 4.0][..], &metric, &ladder).unwrap();
    assert_eq!(values, vec![5.0, 3.0, 2.0]);

    let mut normalized = [0.0; 3];
    normalize_ticks(&values, Normalization::UnitMax, &mut normalized).unwrap();
    assert_eq!(normalized, [1.0, 0.6, 0.4]);

    let mut tanh = [0.0; 3];
    normalize_ticks(&values, Normalization::Tanh, &mut tanh).unwrap();
    assert!(tanh.iter().all(|value| *value > 0.0 && *value <= 1.0));
}

#[test]
fn smooth_surrogate_is_positive_near_threshold() {
    let tier = Tier::new(1.0, 2.0, 0.5, 1.0).unwrap();
    let hard = tick_distance(0.95, tier);
    let params = SmoothParams::sharpness(10.0).unwrap();
    let smooth = smooth_tick_distance(0.95, tier, params).unwrap();
    assert_eq!(hard, 0.0);
    assert!(smooth > 0.0);

    let ladder = geometric_ladder(0.5, 1.0, 2.0, 3, 0.5, 1.0).unwrap();
    let values = smooth_ladder_values(3.0, &ladder, params).unwrap();
    assert_eq!(values.len(), 3);
    assert!(values[0] > values[2]);
    assert!(values
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0));

    let sharper = SmoothParams::sharpness(100.0).unwrap();
    let hard_far = tick_distance(3.0, tier);
    let smooth_far = smooth_tick_distance(3.0, tier, sharper).unwrap();
    assert!((hard_far - smooth_far).abs() < 0.1);

    let mut previous = 0.0;
    for step in 0..100 {
        let value = smooth_tick_distance(step as f64 * 0.05, tier, params).unwrap();
        assert!(value.is_finite() && value >= 0.0);
        assert!(value + 1e-12 >= previous);
        previous = value;
    }

    let h = 1e-5;
    let left = smooth_tick_distance(1.25 - h, tier, params).unwrap();
    let right = smooth_tick_distance(1.25 + h, tier, params).unwrap();
    let finite_difference = (right - left) / (2.0 * h);
    assert!(finite_difference.is_finite());
}

#[test]
fn carry_rules_and_promotion_counter_match_reference_behavior() {
    assert_eq!(carry_rules(&[0.1, 1.2, 3.0]).unwrap(), vec![1, 2, 3]);

    let mut promote = PromotionCounter::new(vec![2, 3]).unwrap();
    let mut flags = [false; 2];
    promote.step(Some(&[false, false]), &mut flags).unwrap();
    assert_eq!(flags, [false, false]);
    assert_eq!(promote.counters(), &[1, 1]);

    promote.step(Some(&[false, false]), &mut flags).unwrap();
    assert_eq!(flags, [true, true]);
    assert_eq!(promote.counters(), &[0, 0]);
}

#[test]
fn event_log_tracks_tier_local_next_events() {
    let mut log = EventLog::new(3).unwrap();
    assert!(log.is_empty());
    assert_eq!(log.append("s0", vec![0.0, 0.0, 0.0]).unwrap(), 0);
    assert_eq!(log.append("s1", vec![1.0, 0.0, 0.0]).unwrap(), 1);
    assert_eq!(log.append("s2", vec![0.0, 2.0, 0.0]).unwrap(), 2);
    assert_eq!(log.append("s3", vec![1.0, 1.0, 0.0]).unwrap(), 3);

    assert_eq!(log.first_event(0), Some(1));
    assert_eq!(log.next_event(1, 0), Some(3));
    assert_eq!(log.next_event(2, 1), Some(3));

    let tier0: Vec<_> = log.iter_events(0).map(|(index, _)| index).collect();
    assert_eq!(tier0, vec![1, 3]);
    let summary = log.compact_summary(1);
    assert_eq!(summary.len(), 2);
    assert_eq!(summary[0].state_id, "s2");
    assert!(log.append("bad", vec![1.0, 0.0]).is_err());
    assert!(EventLog::<u64>::new(0).is_err());
}

#[test]
fn adaptive_zoom_stops_when_coarser_tiers_are_inactive() {
    let ladder = geometric_ladder(0.5, 1.0, 2.0, 4, 0.5, 1.0).unwrap();
    let mut out = [99.0; 4];
    let decision = adaptive_ladder_distance(0.75, &ladder, &mut out).unwrap();
    assert_eq!(decision.first_inactive_tier, Some(1));
    assert!(decision.stopped_early);
    assert!(out[0] > 0.0);
    assert_eq!(&out[1..], &[0.0, 0.0, 0.0]);

    let window = adaptive_zoom_window(3.0, &ladder, 1).unwrap().unwrap();
    assert_eq!(window, 1..4);

    let mut fixed = [99.0; 4];
    let fixed_decision =
        zoom_ladder_distance(3.0, &ladder, ZoomPolicy::FixedDepth(2), &mut fixed).unwrap();
    assert_eq!(fixed_decision.evaluated_tiers, 2);
    assert!(fixed[0] > 0.0 && fixed[1] > 0.0);
    assert_eq!(&fixed[2..], &[0.0, 0.0]);

    let mut capped = [99.0; 4];
    let capped_decision =
        zoom_ladder_distance(0.75, &ladder, ZoomPolicy::DepthCap(2), &mut capped).unwrap();
    assert_eq!(capped_decision.first_inactive_tier, Some(1));
    assert_eq!(capped_decision.evaluated_tiers, 2);
    assert_eq!(&capped[1..], &[0.0, 0.0, 0.0]);

    let mut full = [0.0; 4];
    let full_decision =
        zoom_ladder_distance(10.0, &ladder, ZoomPolicy::EarlyStop, &mut full).unwrap();
    assert_eq!(full_decision.evaluated_tiers, 4);
    assert!(!full_decision.stopped_early);
    assert!(full.iter().all(|value| *value > 0.0));

    assert!(zoom_ladder_distance(3.0, &ladder, ZoomPolicy::DepthCap(5), &mut fixed).is_err());
    assert!(zoom_ladder_distance(-1.0, &ladder, ZoomPolicy::EarlyStop, &mut fixed).is_err());
}

#[test]
fn minimal_consensus_computes_residuals_and_weight_updates() {
    let a = [1.0, 2.0, 0.0];
    let b = [3.0, 0.0, 0.0];
    let mut consensus = [0.0; 3];
    let meta = weighted_consensus(&[&a, &b], &[0.25, 0.75], &mut consensus).unwrap();
    assert_eq!(meta.sources, 2);
    assert_eq!(consensus, [2.5, 0.5, 0.0]);

    let residual = coherence_residual(&a, &consensus).unwrap();
    assert!(residual > 0.0);
    assert_eq!(coherence_residual(&consensus, &consensus).unwrap(), 0.0);

    let mut residuals = [0.0; 2];
    coherence_residuals(&[&a, &b], &consensus, &mut residuals).unwrap();
    let mut weights = [0.5, 0.5];
    simple_weight_update(&mut weights, &residuals, 0.2, 0.01).unwrap();
    assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    assert!(weights[0] < 0.5 || weights[1] < 0.5);

    let mut out = [0.0; 2];
    assert!(weighted_consensus(&[], &[], &mut out).is_err());
    assert!(weighted_consensus(&[&a[..]], &[0.0], &mut out).is_err());
    assert!(weighted_consensus(&[&a[..]], &[-1.0], &mut out).is_err());
    assert!(weighted_consensus(&[&[][..]], &[1.0], &mut out).is_err());
}

#[test]
fn deterministic_property_grid_matches_core_contracts() {
    for epsilon_index in 1..6 {
        for delta_index in epsilon_index + 1..epsilon_index + 7 {
            for p in [-1.0, 0.0, 0.5, 1.0] {
                let epsilon = epsilon_index as f64 * 0.1;
                let delta = delta_index as f64 * 0.1;
                let tier = Tier::new(epsilon, delta, p, 1.0).unwrap();
                let mut previous = 0.0;
                for distance_index in 0..80 {
                    let distance = distance_index as f64 * 0.05;
                    let tick = try_tick_distance(distance, tier).unwrap();
                    assert!(tick >= 0.0);
                    assert!(tick + 1e-12 >= previous);
                    if distance < epsilon {
                        assert_eq!(tick, 0.0);
                    }
                    if p == 0.0 && distance >= epsilon {
                        assert_eq!(tick, (distance / delta).ceil());
                    }
                    previous = tick;
                }
            }
        }
    }

    let ladder = Ladder::geometric(0.1, 0.2, 2.0, 5, 0.0, 1.0).unwrap();
    for distance_index in 0..50 {
        assert_eq!(
            ladder.values(distance_index as f64 * 0.1).unwrap().len(),
            ladder.len()
        );
    }
}
