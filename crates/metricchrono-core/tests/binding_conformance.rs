use std::collections::BTreeMap;

use metricchrono_core::{
    adaptive_ladder_distance, carry_rules, custom_ladder, geometric_ladder, ladder_distance,
    ladder_pair, ladder_values, normalize_ticks, smooth_ladder_distance, smooth_tick_distance,
    tick_distance, tick_pair, try_tick_distance, validate_ladder, weighted_consensus, Absolute,
    Euclidean, EventLog, Ladder, Metric, MetricFn, Normalization, PromotionCounter, SmoothParams,
    Tier, TierDocument,
};
use serde_json::Value;

#[test]
fn rust_core_matches_binding_conformance_fixture() {
    let fixture = fixture();
    let tolerance = f64_field(&fixture, "tolerance");
    assert_eq!(
        str_field(&fixture, "metricchrono_schema"),
        "binding_conformance.v1"
    );

    let mut ladders = BTreeMap::new();
    for case in array_field(&fixture, "ladders") {
        let name = str_field(case, "name").to_owned();
        let expected_tiers = tiers_from_value(field(case, "tiers"));
        let actual = match str_field(case, "kind") {
            "geometric" => {
                let params = field(case, "params");
                geometric_ladder(
                    f64_field(params, "epsilon0"),
                    f64_field(params, "delta0"),
                    f64_field(params, "ratio"),
                    usize_field(params, "tiers"),
                    f64_field(params, "p"),
                    f64_field(params, "epsilon_ref"),
                )
                .unwrap()
            }
            "custom" => custom_ladder(expected_tiers.clone()).unwrap(),
            other => panic!("unknown ladder kind {other}"),
        };
        assert_tiers_close(&name, &actual, &expected_tiers, tolerance);
        validate_ladder(&actual).unwrap();

        let ladder = Ladder::new(actual.clone()).unwrap();
        for distance_case in array_field(case, "distances") {
            let distance = f64_field(distance_case, "distance");
            let expected = f64_vec(field(distance_case, "expected"));
            let mut out = vec![0.0; actual.len()];
            ladder_distance(distance, &actual, &mut out).unwrap();
            assert_vec_close(
                &format!("{name}/ladder_distance"),
                &out,
                &expected,
                tolerance,
            );
            assert_vec_close(
                &format!("{name}/ladder_values"),
                &ladder_values(distance, &actual).unwrap(),
                &expected,
                tolerance,
            );
            assert_vec_close(
                &format!("{name}/Ladder::values"),
                &ladder.values(distance).unwrap(),
                &expected,
                tolerance,
            );
        }
        ladders.insert(name, actual);
    }

    for case in array_field(&fixture, "tick_distance_cases") {
        let name = str_field(case, "name");
        let tier = tier_from_value(field(case, "tier"));
        let distance = f64_field(case, "distance");
        let expected = f64_field(case, "expected");
        assert_close(
            &format!("{name}/tick_distance"),
            tick_distance(distance, tier),
            expected,
            tolerance,
        );
        assert_close(
            &format!("{name}/try_tick_distance"),
            try_tick_distance(distance, tier).unwrap(),
            expected,
            tolerance,
        );
    }

    for case in array_field(&fixture, "smooth_tick_distance_cases") {
        let expected = f64_field(case, "expected");
        let actual = smooth_tick_distance(
            f64_field(case, "distance"),
            tier_from_value(field(case, "tier")),
            SmoothParams::sharpness(f64_field(case, "sharpness")).unwrap(),
        )
        .unwrap();
        assert_close(str_field(case, "name"), actual, expected, tolerance);
    }

    for case in array_field(&fixture, "smooth_ladder_distance_cases") {
        let ladder = ladder_named(&ladders, str_field(case, "ladder"));
        let expected = f64_vec(field(case, "expected"));
        let mut out = vec![0.0; ladder.len()];
        smooth_ladder_distance(
            f64_field(case, "distance"),
            ladder,
            SmoothParams::sharpness(f64_field(case, "sharpness")).unwrap(),
            &mut out,
        )
        .unwrap();
        assert_vec_close(str_field(case, "name"), &out, &expected, tolerance);
    }

    for case in array_field(&fixture, "adaptive_ladder_distance_cases") {
        let ladder = ladder_named(&ladders, str_field(case, "ladder"));
        let mut ticks = vec![0.0; ladder.len()];
        let decision =
            adaptive_ladder_distance(f64_field(case, "distance"), ladder, &mut ticks).unwrap();
        let expected = field(case, "expected");
        assert_vec_close(
            str_field(case, "name"),
            &ticks,
            &f64_vec(field(expected, "ticks")),
            tolerance,
        );
        let expected_decision = field(expected, "decision");
        assert_eq!(
            decision.evaluated_tiers,
            usize_field(expected_decision, "evaluated_tiers")
        );
        assert_eq!(
            decision.first_inactive_tier,
            optional_usize(field(expected_decision, "first_inactive_tier"))
        );
        assert_eq!(
            decision.stopped_early,
            bool_field(expected_decision, "stopped_early")
        );
    }

    for case in array_field(&fixture, "weighted_consensus_cases") {
        let vectors = f64_matrix(field(case, "vectors"));
        let refs = vectors.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let weights = f64_vec(field(case, "weights"));
        let mut out = vec![0.0; vectors[0].len()];
        let result = weighted_consensus(&refs, &weights, &mut out).unwrap();
        assert_vec_close(
            str_field(case, "name"),
            &out,
            &f64_vec(field(case, "expected")),
            tolerance,
        );
        let expected_result = field(case, "result");
        assert_eq!(result.sources, usize_field(expected_result, "sources"));
        assert_eq!(result.tiers, usize_field(expected_result, "tiers"));
        assert_close(
            "weighted_consensus total_weight",
            result.total_weight,
            f64_field(expected_result, "total_weight"),
            tolerance,
        );
    }

    for case in array_field(&fixture, "event_log_cases") {
        assert_event_log_case(case, tolerance);
    }

    assert_metric_cases(field(&fixture, "metric_cases"), tolerance);
    assert_pair_cases(field(&fixture, "pair_cases"), &ladders, tolerance);

    for case in array_field(&fixture, "normalize_ticks_cases") {
        let input = f64_vec(field(case, "input"));
        let mut out = vec![0.0; input.len()];
        normalize_ticks(&input, normalization(str_field(case, "mode")), &mut out).unwrap();
        assert_vec_close(
            str_field(case, "name"),
            &out,
            &f64_vec(field(case, "expected")),
            tolerance,
        );
    }

    for case in array_field(&fixture, "carry_rules_cases") {
        assert_eq!(
            carry_rules(&f64_vec(field(case, "epsilons"))).unwrap(),
            u64_vec(field(case, "expected")),
            "{}",
            str_field(case, "name")
        );
    }

    for case in array_field(&fixture, "promotion_counter_cases") {
        let quotas = u64_vec(field(case, "quotas"));
        let mut counter = PromotionCounter::new(quotas.clone()).unwrap();
        assert_eq!(counter.quotas(), quotas.as_slice());
        for step in array_field(case, "steps") {
            let flags = optional_bool_vec(field(step, "event_flags"));
            let mut promoted = vec![false; counter.len()];
            counter.step(flags.as_deref(), &mut promoted).unwrap();
            assert_eq!(promoted, bool_vec(field(step, "promoted")));
            assert_eq!(
                counter.counters(),
                u64_vec(field(step, "counters")).as_slice()
            );
        }
        counter.reset();
        assert_eq!(
            counter.counters(),
            u64_vec(field(case, "after_reset_counters")).as_slice()
        );
    }

    assert_rejections(
        field(&fixture, "rejections"),
        &array_field(&fixture, "event_log_cases")[0],
    );
}

fn assert_event_log_case(case: &Value, tolerance: f64) {
    let mut log = EventLog::new(usize_field(case, "tier_count")).unwrap();
    assert_eq!(log.is_empty(), bool_field(case, "is_empty_before_append"));
    for record in array_field(case, "append_records") {
        assert_eq!(
            log.append(
                u64_field(record, "state_id"),
                f64_vec(field(record, "ticks"))
            )
            .unwrap(),
            usize_field(record, "expected_index")
        );
    }
    assert_eq!(log.len(), usize_field(case, "expected_len"));
    assert_eq!(log.is_empty(), bool_field(case, "is_empty_after_append"));
    assert_eq!(log.tier_count(), usize_field(case, "tier_count"));

    for expected in array_field(case, "records") {
        let index = usize_field(expected, "index");
        let actual = log.record(index).unwrap();
        assert_eq!(actual.state_id, u64_field(expected, "state_id"));
        assert_vec_close(
            &format!("record {index}"),
            &actual.ticks,
            &f64_vec(field(expected, "ticks")),
            tolerance,
        );
    }
    for expected in array_field(case, "first_events") {
        assert_eq!(
            log.first_event(usize_field(expected, "tier")),
            optional_usize(field(expected, "expected"))
        );
    }
    for expected in array_field(case, "next_events") {
        assert_eq!(
            log.next_event(
                usize_field(expected, "index"),
                usize_field(expected, "tier")
            ),
            optional_usize(field(expected, "expected"))
        );
    }
    for expected in array_field(case, "compact_summaries") {
        let tier = usize_field(expected, "tier");
        let actual = log.compact_summary(tier);
        let expected_items = array_field(expected, "expected");
        assert_eq!(
            actual.len(),
            expected_items.len(),
            "compact summary tier {tier}"
        );
        for (actual, expected) in actual.iter().zip(expected_items) {
            assert_eq!(actual.index, usize_field(expected, "index"));
            assert_eq!(actual.state_id, u64_field(expected, "state_id"));
            assert_close(
                "compact summary tick",
                actual.tick,
                f64_field(expected, "tick"),
                tolerance,
            );
        }
    }
}

fn assert_metric_cases(cases: &Value, tolerance: f64) {
    for case in array_field(cases, "euclidean_distance") {
        let a = f64_vec(field(case, "a"));
        let b = f64_vec(field(case, "b"));
        assert_close(
            str_field(case, "name"),
            Euclidean.distance(a.as_slice(), b.as_slice()),
            f64_field(case, "expected"),
            tolerance,
        );
    }
    for case in array_field(cases, "absolute_distance") {
        assert_close(
            str_field(case, "name"),
            Absolute.distance(&f64_field(case, "a"), &f64_field(case, "b")),
            f64_field(case, "expected"),
            tolerance,
        );
    }
}

fn assert_pair_cases(cases: &Value, ladders: &BTreeMap<String, Vec<Tier>>, tolerance: f64) {
    for case in array_field(cases, "tick_pair") {
        let tier = tier_from_value(field(case, "tier"));
        let actual = match str_field(case, "metric") {
            "euclidean" => {
                let a = f64_vec(field(case, "a"));
                let b = f64_vec(field(case, "b"));
                tick_pair(a.as_slice(), b.as_slice(), &Euclidean, tier).unwrap()
            }
            "absolute" => tick_pair(
                &f64_field(case, "a"),
                &f64_field(case, "b"),
                &Absolute,
                tier,
            )
            .unwrap(),
            "max_abs" => {
                let a = f64_vec(field(case, "a"));
                let b = f64_vec(field(case, "b"));
                tick_pair(
                    a.as_slice(),
                    b.as_slice(),
                    &MetricFn(max_abs_distance),
                    tier,
                )
                .unwrap()
            }
            other => panic!("unknown metric {other}"),
        };
        assert_close(
            str_field(case, "name"),
            actual,
            f64_field(case, "expected"),
            tolerance,
        );
    }

    for case in array_field(cases, "ladder_pair") {
        let ladder = ladder_named(ladders, str_field(case, "ladder"));
        let actual = match str_field(case, "metric") {
            "euclidean" => {
                let a = f64_vec(field(case, "a"));
                let b = f64_vec(field(case, "b"));
                ladder_pair(a.as_slice(), b.as_slice(), &Euclidean, ladder).unwrap()
            }
            "absolute" => ladder_pair(
                &f64_field(case, "a"),
                &f64_field(case, "b"),
                &Absolute,
                ladder,
            )
            .unwrap(),
            "max_abs" => {
                let a = f64_vec(field(case, "a"));
                let b = f64_vec(field(case, "b"));
                ladder_pair(
                    a.as_slice(),
                    b.as_slice(),
                    &MetricFn(max_abs_distance),
                    ladder,
                )
                .unwrap()
            }
            other => panic!("unknown metric {other}"),
        };
        assert_vec_close(
            str_field(case, "name"),
            &actual,
            &f64_vec(field(case, "expected")),
            tolerance,
        );
    }
}

fn assert_rejections(rejections: &Value, event_case: &Value) {
    for case in array_field(rejections, "invalid_tiers") {
        assert!(
            Tier::new(
                f64_field(case, "epsilon"),
                f64_field(case, "delta"),
                f64_field(case, "p"),
                f64_field(case, "epsilon_ref"),
            )
            .is_err(),
            "{} should reject",
            str_field(case, "name")
        );
    }

    for case in array_field(rejections, "unknown_schema_documents") {
        match str_field(case, "kind") {
            "tier" => {
                let parsed =
                    serde_json::from_value::<TierDocument>(field(case, "document").clone());
                assert!(parsed.is_err(), "{} should reject", str_field(case, "name"));
            }
            other => panic!("unknown schema rejection kind {other}"),
        }
    }

    let len = usize_field(event_case, "expected_len");
    let tier_count = usize_field(event_case, "tier_count");
    for case in array_field(rejections, "event_log_out_of_range") {
        let index = usize_field(case, "index");
        let tier = usize_field(case, "tier");
        assert!(
            index >= len || tier >= tier_count,
            "{} must be out of range",
            str_field(case, "name")
        );
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/binding_conformance.v1.json")).unwrap()
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    value
        .get(name)
        .unwrap_or_else(|| panic!("missing fixture field {name}"))
}

fn array_field<'a>(value: &'a Value, name: &str) -> &'a Vec<Value> {
    field(value, name)
        .as_array()
        .unwrap_or_else(|| panic!("fixture field {name} must be an array"))
}

fn str_field<'a>(value: &'a Value, name: &str) -> &'a str {
    field(value, name)
        .as_str()
        .unwrap_or_else(|| panic!("fixture field {name} must be a string"))
}

fn f64_field(value: &Value, name: &str) -> f64 {
    field(value, name)
        .as_f64()
        .unwrap_or_else(|| panic!("fixture field {name} must be a number"))
}

fn u64_field(value: &Value, name: &str) -> u64 {
    field(value, name)
        .as_u64()
        .unwrap_or_else(|| panic!("fixture field {name} must be a u64"))
}

fn usize_field(value: &Value, name: &str) -> usize {
    u64_field(value, name) as usize
}

fn bool_field(value: &Value, name: &str) -> bool {
    field(value, name)
        .as_bool()
        .unwrap_or_else(|| panic!("fixture field {name} must be a bool"))
}

fn tier_from_value(value: &Value) -> Tier {
    Tier::new(
        f64_field(value, "epsilon"),
        f64_field(value, "delta"),
        f64_field(value, "p"),
        f64_field(value, "epsilon_ref"),
    )
    .unwrap()
}

fn tiers_from_value(value: &Value) -> Vec<Tier> {
    value
        .as_array()
        .expect("tiers must be an array")
        .iter()
        .map(tier_from_value)
        .collect()
}

fn f64_vec(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("expected number array")
        .iter()
        .map(|item| item.as_f64().expect("expected number"))
        .collect()
}

fn f64_matrix(value: &Value) -> Vec<Vec<f64>> {
    value
        .as_array()
        .expect("expected matrix")
        .iter()
        .map(f64_vec)
        .collect()
}

fn u64_vec(value: &Value) -> Vec<u64> {
    value
        .as_array()
        .expect("expected u64 array")
        .iter()
        .map(|item| item.as_u64().expect("expected u64"))
        .collect()
}

fn bool_vec(value: &Value) -> Vec<bool> {
    value
        .as_array()
        .expect("expected bool array")
        .iter()
        .map(|item| item.as_bool().expect("expected bool"))
        .collect()
}

fn optional_bool_vec(value: &Value) -> Option<Vec<bool>> {
    if value.is_null() {
        None
    } else {
        Some(bool_vec(value))
    }
}

fn optional_usize(value: &Value) -> Option<usize> {
    if value.is_null() {
        None
    } else {
        Some(value.as_u64().expect("expected optional usize") as usize)
    }
}

fn normalization(mode: &str) -> Normalization {
    match mode {
        "none" => Normalization::None,
        "unit_max" => Normalization::UnitMax,
        "tanh" => Normalization::Tanh,
        other => panic!("unknown normalization {other}"),
    }
}

fn ladder_named<'a>(ladders: &'a BTreeMap<String, Vec<Tier>>, name: &str) -> &'a [Tier] {
    ladders
        .get(name)
        .unwrap_or_else(|| panic!("missing ladder {name}"))
}

fn assert_tiers_close(name: &str, actual: &[Tier], expected: &[Tier], tolerance: f64) {
    assert_eq!(actual.len(), expected.len(), "{name} tier length");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_close(
            &format!("{name}[{index}].epsilon"),
            actual.epsilon,
            expected.epsilon,
            tolerance,
        );
        assert_close(
            &format!("{name}[{index}].delta"),
            actual.delta,
            expected.delta,
            tolerance,
        );
        assert_close(
            &format!("{name}[{index}].p"),
            actual.p,
            expected.p,
            tolerance,
        );
        assert_close(
            &format!("{name}[{index}].epsilon_ref"),
            actual.epsilon_ref,
            expected.epsilon_ref,
            tolerance,
        );
    }
}

fn assert_vec_close(name: &str, actual: &[f64], expected: &[f64], tolerance: f64) {
    assert_eq!(actual.len(), expected.len(), "{name} length");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_close(&format!("{name}[{index}]"), *actual, *expected, tolerance);
    }
}

fn assert_close(name: &str, actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{name}: expected {expected}, got {actual}"
    );
}

fn max_abs_distance(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return f64::NAN;
    }
    a.iter()
        .zip(b)
        .map(|(left, right)| (left - right).abs())
        .fold(0.0, f64::max)
}
