use std::{env, fs, path::PathBuf};

use metricchrono_core::{
    adaptive_ladder_distance, carry_rules, custom_ladder, geometric_ladder, ladder_pair,
    ladder_values, normalize_ticks, smooth_ladder_values, smooth_tick_distance, tick_distance,
    tick_pair, weighted_consensus, Absolute, Euclidean, EventLog, Ladder, Metric, MetricFn,
    Normalization, PromotionCounter, SmoothParams, Tier,
};
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = build_fixture()?;
    let output = env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/binding_conformance.v1.json")
    });

    let mut serialized = serde_json::to_string_pretty(&fixture)?;
    serialized.push('\n');
    fs::write(output, serialized)?;
    Ok(())
}

fn build_fixture() -> Result<Value, Box<dyn std::error::Error>> {
    let paper = Tier::new(0.1, 0.3, 0.5, 1.0)?;
    let weighted = Tier::new(0.5, 1.0, 0.5, 1.0)?;
    let flat = Tier::new(0.5, 1.0, 0.0, 1.0)?;
    let fractional = Tier::new(0.25, 0.75, 0.25, 0.5)?;

    let geometric_paper = geometric_ladder(0.03, 0.10, 3.0, 3, 0.0, 1.0)?;
    let geometric_weighted = geometric_ladder(0.5, 1.0, 2.0, 4, 0.5, 1.0)?;
    let custom_four = custom_ladder(vec![
        Tier::new(0.2, 0.5, 0.0, 1.0)?,
        Tier::new(0.7, 1.4, 0.25, 1.0)?,
        Tier::new(1.5, 3.0, 0.5, 1.0)?,
        Tier::new(4.0, 8.0, 0.75, 2.0)?,
    ])?;

    let tick_distance_cases = vec![
        tick_case("paper_below_threshold", 0.05, paper),
        tick_case("paper_active_one_stair", 0.20, paper),
        tick_case("paper_active_two_stairs", 0.60, paper),
        tick_case("weighted_at_threshold", 0.50, weighted),
        tick_case("flat_unweighted", 5.00, flat),
        tick_case("fractional_gain", 2.25, fractional),
    ];

    let smooth_tick_distance_cases = vec![
        smooth_tick_case("soft_sharpness_4", 0.95, weighted, 4.0)?,
        smooth_tick_case("harder_sharpness_20", 0.95, weighted, 20.0)?,
        smooth_tick_case("fractional_sharpness_8", 1.10, fractional, 8.0)?,
    ];

    let smooth_ladder_distance_cases = vec![
        smooth_ladder_case(
            "weighted_smooth_4",
            2.25,
            "geometric_weighted",
            &geometric_weighted,
            4.0,
        )?,
        smooth_ladder_case("custom_smooth_12", 3.50, "custom_four", &custom_four, 12.0)?,
    ];

    let adaptive_ladder_distance_cases = vec![
        adaptive_ladder_case(
            "early_stop_weighted",
            0.75,
            "geometric_weighted",
            &geometric_weighted,
        )?,
        adaptive_ladder_case(
            "full_weighted",
            9.50,
            "geometric_weighted",
            &geometric_weighted,
        )?,
        adaptive_ladder_case("custom_middle_stop", 2.00, "custom_four", &custom_four)?,
    ];

    let consensus_vectors = vec![
        vec![1.0, 2.0, 0.0, 4.0],
        vec![3.0, 0.0, 6.0, 2.0],
        vec![0.0, 8.0, 2.0, 1.0],
    ];
    let consensus_weights = vec![0.25, 0.50, 0.25];
    let weighted_consensus_cases = vec![weighted_consensus_case(
        "three_sources_four_tiers",
        consensus_vectors,
        consensus_weights,
    )?];

    let event_log_cases = vec![event_log_case()?];

    let metric_cases = json!({
        "euclidean_distance": [
            {
                "name": "three_four_five",
                "a": [0.0, 0.0],
                "b": [3.0, 4.0],
                "expected": Euclidean.distance(&[0.0, 0.0], &[3.0, 4.0]),
            },
            {
                "name": "signed_three_dimensional",
                "a": [-1.0, 2.5, 4.0],
                "b": [2.0, -1.5, 4.0],
                "expected": Euclidean.distance(&[-1.0, 2.5, 4.0], &[2.0, -1.5, 4.0]),
            }
        ],
        "absolute_distance": [
            {
                "name": "scalar_gap",
                "a": 2.0,
                "b": 5.5,
                "expected": Absolute.distance(&2.0, &5.5),
            },
            {
                "name": "signed_scalar_gap",
                "a": -3.25,
                "b": 1.75,
                "expected": Absolute.distance(&-3.25, &1.75),
            }
        ]
    });

    let pair_cases = json!({
        "tick_pair": [
            tick_pair_case("euclidean_tick_pair", "euclidean", json!([0.0, 0.0]), json!([3.0, 4.0]), weighted)?,
            tick_pair_case("absolute_tick_pair", "absolute", json!(2.0), json!(5.5), weighted)?,
            custom_tick_pair_case("metric_fn_tick_pair", json!([1.0, -2.0, 4.0]), json!([2.5, 1.0, 1.0]), weighted)?,
        ],
        "ladder_pair": [
            ladder_pair_case("euclidean_ladder_pair", "euclidean", json!([0.0, 0.0]), json!([3.0, 4.0]), "geometric_weighted", &geometric_weighted)?,
            ladder_pair_case("absolute_ladder_pair", "absolute", json!(2.0), json!(5.5), "custom_four", &custom_four)?,
            custom_ladder_pair_case("metric_fn_ladder_pair", json!([1.0, -2.0, 4.0]), json!([2.5, 1.0, 1.0]), "custom_four", &custom_four)?,
        ]
    });

    let normalize_ticks_cases = vec![
        normalize_case("none", "none", &[10.0, -5.0, 0.0, 2.5])?,
        normalize_case("unit_max", "unit_max", &[10.0, -5.0, 0.0, 2.5])?,
        normalize_case("tanh", "tanh", &[1.0, -1.0, 0.0, 3.0])?,
        normalize_case("unit_max_all_zero", "unit_max", &[0.0, 0.0, 0.0])?,
    ];

    let carry_rules_cases = vec![
        carry_case("mixed_epsilons", &[0.1, 1.2, 3.0, 4.01])?,
        carry_case("integer_epsilons", &[1.0, 2.0, 5.0])?,
    ];

    let promotion_counter_cases = vec![promotion_case()?];

    Ok(json!({
        "metricchrono_schema": "binding_conformance.v1",
        "tolerance": 1.0e-12,
        "tiers": [
            named_tier("paper", paper),
            named_tier("weighted", weighted),
            named_tier("flat", flat),
            named_tier("fractional", fractional),
        ],
        "ladders": [
            geometric_ladder_case("geometric_paper", 0.03, 0.10, 3.0, 3, 0.0, 1.0, &geometric_paper)?,
            geometric_ladder_case("geometric_weighted", 0.5, 1.0, 2.0, 4, 0.5, 1.0, &geometric_weighted)?,
            custom_ladder_case("custom_four", &custom_four)?,
        ],
        "tick_distance_cases": tick_distance_cases,
        "smooth_tick_distance_cases": smooth_tick_distance_cases,
        "smooth_ladder_distance_cases": smooth_ladder_distance_cases,
        "adaptive_ladder_distance_cases": adaptive_ladder_distance_cases,
        "weighted_consensus_cases": weighted_consensus_cases,
        "event_log_cases": event_log_cases,
        "metric_cases": metric_cases,
        "pair_cases": pair_cases,
        "normalize_ticks_cases": normalize_ticks_cases,
        "carry_rules_cases": carry_rules_cases,
        "promotion_counter_cases": promotion_counter_cases,
        "rejections": {
            "invalid_tiers": [
                {
                    "name": "epsilon_equal_delta",
                    "epsilon": 1.0,
                    "delta": 1.0,
                    "p": 0.0,
                    "epsilon_ref": 1.0
                },
                {
                    "name": "epsilon_greater_delta",
                    "epsilon": 2.0,
                    "delta": 1.0,
                    "p": 0.0,
                    "epsilon_ref": 1.0
                }
            ],
            "unknown_schema_documents": [
                {
                    "name": "tier_unknown_field",
                    "kind": "tier",
                    "document": {
                        "metricchrono_schema": "tier.v1",
                        "epsilon": 0.03,
                        "delta": 0.1,
                        "p": 0.0,
                        "epsilon_ref": 1.0,
                        "unknown": true
                    }
                }
            ],
            "event_log_out_of_range": [
                {
                    "name": "record_index_too_large",
                    "operation": "record",
                    "index": 99,
                    "tier": 0
                },
                {
                    "name": "next_event_index_too_large",
                    "operation": "next_event",
                    "index": 99,
                    "tier": 0
                },
                {
                    "name": "first_event_tier_too_large",
                    "operation": "first_event",
                    "index": 0,
                    "tier": 3
                },
                {
                    "name": "compact_summary_tier_too_large",
                    "operation": "compact_summary",
                    "index": 0,
                    "tier": 3
                }
            ]
        }
    }))
}

fn named_tier(name: &str, tier: Tier) -> Value {
    json!({
        "name": name,
        "epsilon": tier.epsilon,
        "delta": tier.delta,
        "p": tier.p,
        "epsilon_ref": tier.epsilon_ref,
    })
}

fn tier_value(tier: Tier) -> Value {
    json!({
        "epsilon": tier.epsilon,
        "delta": tier.delta,
        "p": tier.p,
        "epsilon_ref": tier.epsilon_ref,
    })
}

fn tiers_value(ladder: &[Tier]) -> Value {
    Value::Array(ladder.iter().copied().map(tier_value).collect())
}

fn tick_case(name: &str, distance: f64, tier: Tier) -> Value {
    json!({
        "name": name,
        "distance": distance,
        "tier": tier_value(tier),
        "expected": tick_distance(distance, tier),
    })
}

#[allow(clippy::too_many_arguments)]
fn geometric_ladder_case(
    name: &str,
    epsilon0: f64,
    delta0: f64,
    ratio: f64,
    tiers: usize,
    p: f64,
    epsilon_ref: f64,
    ladder: &[Tier],
) -> Result<Value, Box<dyn std::error::Error>> {
    let ladder_object = Ladder::geometric(epsilon0, delta0, ratio, tiers, p, epsilon_ref)?;
    Ok(json!({
        "name": name,
        "kind": "geometric",
        "params": {
            "epsilon0": epsilon0,
            "delta0": delta0,
            "ratio": ratio,
            "tiers": tiers,
            "p": p,
            "epsilon_ref": epsilon_ref,
        },
        "tiers": tiers_value(ladder_object.tiers()),
        "distances": ladder_distances(ladder)?,
    }))
}

fn custom_ladder_case(name: &str, ladder: &[Tier]) -> Result<Value, Box<dyn std::error::Error>> {
    let ladder_object = Ladder::new(ladder.to_vec())?;
    Ok(json!({
        "name": name,
        "kind": "custom",
        "tiers": tiers_value(ladder_object.tiers()),
        "distances": ladder_distances(ladder)?,
    }))
}

fn ladder_distances(ladder: &[Tier]) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    [0.05, 0.20, 1.00, 3.00, 9.50]
        .into_iter()
        .map(|distance| {
            Ok(json!({
                "distance": distance,
                "expected": ladder_values(distance, ladder)?,
            }))
        })
        .collect()
}

fn smooth_tick_case(
    name: &str,
    distance: f64,
    tier: Tier,
    sharpness: f64,
) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "name": name,
        "distance": distance,
        "tier": tier_value(tier),
        "sharpness": sharpness,
        "expected": smooth_tick_distance(distance, tier, SmoothParams::sharpness(sharpness)?)?,
    }))
}

fn smooth_ladder_case(
    name: &str,
    distance: f64,
    ladder_name: &str,
    ladder: &[Tier],
    sharpness: f64,
) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "name": name,
        "distance": distance,
        "ladder": ladder_name,
        "sharpness": sharpness,
        "expected": smooth_ladder_values(distance, ladder, SmoothParams::sharpness(sharpness)?)?,
    }))
}

fn adaptive_ladder_case(
    name: &str,
    distance: f64,
    ladder_name: &str,
    ladder: &[Tier],
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut ticks = vec![0.0; ladder.len()];
    let decision = adaptive_ladder_distance(distance, ladder, &mut ticks)?;
    Ok(json!({
        "name": name,
        "distance": distance,
        "ladder": ladder_name,
        "expected": {
            "ticks": ticks,
            "decision": {
                "evaluated_tiers": decision.evaluated_tiers,
                "first_inactive_tier": decision.first_inactive_tier,
                "stopped_early": decision.stopped_early,
            }
        }
    }))
}

fn weighted_consensus_case(
    name: &str,
    vectors: Vec<Vec<f64>>,
    weights: Vec<f64>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let refs = vectors.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let mut out = vec![0.0; vectors[0].len()];
    let result = weighted_consensus(&refs, &weights, &mut out)?;
    Ok(json!({
        "name": name,
        "vectors": vectors,
        "weights": weights,
        "expected": out,
        "result": {
            "sources": result.sources,
            "tiers": result.tiers,
            "total_weight": result.total_weight,
        }
    }))
}

fn event_log_case() -> Result<Value, Box<dyn std::error::Error>> {
    let input_records = vec![
        (100_u64, vec![0.0, 0.0, 0.0]),
        (101_u64, vec![1.0, 0.0, 0.0]),
        (102_u64, vec![0.0, 0.0, 0.0]),
        (103_u64, vec![0.0, 2.0, 0.0]),
        (104_u64, vec![0.0, 0.0, 3.0]),
        (105_u64, vec![4.0, 5.0, 0.0]),
        (106_u64, vec![0.0, 0.0, 0.0]),
        (107_u64, vec![0.0, 0.0, 6.0]),
    ];
    let mut log = EventLog::new(3)?;
    let is_empty_before_append = log.is_empty();
    let mut appended = Vec::new();
    for (state_id, ticks) in &input_records {
        let index = log.append(*state_id, ticks.clone())?;
        appended.push(json!({
            "state_id": state_id,
            "ticks": ticks,
            "expected_index": index,
        }));
    }

    let mut records = Vec::new();
    for index in 0..log.len() {
        let record = log.record(index).expect("record exists after append");
        records.push(json!({
            "index": index,
            "state_id": record.state_id,
            "ticks": record.ticks,
        }));
    }

    let mut first_events = Vec::new();
    let mut compact_summaries = Vec::new();
    for tier in 0..log.tier_count() {
        first_events.push(json!({
            "tier": tier,
            "expected": log.first_event(tier),
        }));
        compact_summaries.push(json!({
            "tier": tier,
            "expected": log
                .compact_summary(tier)
                .into_iter()
                .map(|item| json!({
                    "index": item.index,
                    "state_id": item.state_id,
                    "tick": item.tick,
                }))
                .collect::<Vec<_>>(),
        }));
    }

    let mut next_events = Vec::new();
    for index in 0..log.len() {
        for tier in 0..log.tier_count() {
            next_events.push(json!({
                "index": index,
                "tier": tier,
                "expected": log.next_event(index, tier),
            }));
        }
    }

    Ok(json!({
        "name": "sparse_quiet_and_event_records",
        "tier_count": log.tier_count(),
        "is_empty_before_append": is_empty_before_append,
        "is_empty_after_append": log.is_empty(),
        "expected_len": log.len(),
        "append_records": appended,
        "records": records,
        "first_events": first_events,
        "next_events": next_events,
        "compact_summaries": compact_summaries,
    }))
}

fn tick_pair_case(
    name: &str,
    metric: &str,
    a: Value,
    b: Value,
    tier: Tier,
) -> Result<Value, Box<dyn std::error::Error>> {
    let expected = match metric {
        "euclidean" => {
            let a = number_array(&a);
            let b = number_array(&b);
            tick_pair(a.as_slice(), b.as_slice(), &Euclidean, tier)?
        }
        "absolute" => tick_pair(&number(&a), &number(&b), &Absolute, tier)?,
        _ => unreachable!("fixture generator only uses known metrics"),
    };
    Ok(json!({
        "name": name,
        "metric": metric,
        "a": a,
        "b": b,
        "tier": tier_value(tier),
        "expected": expected,
    }))
}

fn ladder_pair_case(
    name: &str,
    metric: &str,
    a: Value,
    b: Value,
    ladder_name: &str,
    ladder: &[Tier],
) -> Result<Value, Box<dyn std::error::Error>> {
    let expected = match metric {
        "euclidean" => {
            let a = number_array(&a);
            let b = number_array(&b);
            ladder_pair(a.as_slice(), b.as_slice(), &Euclidean, ladder)?
        }
        "absolute" => ladder_pair(&number(&a), &number(&b), &Absolute, ladder)?,
        _ => unreachable!("fixture generator only uses known metrics"),
    };
    Ok(json!({
        "name": name,
        "metric": metric,
        "a": a,
        "b": b,
        "ladder": ladder_name,
        "expected": expected,
    }))
}

fn custom_tick_pair_case(
    name: &str,
    a: Value,
    b: Value,
    tier: Tier,
) -> Result<Value, Box<dyn std::error::Error>> {
    let a_values = number_array(&a);
    let b_values = number_array(&b);
    let metric = MetricFn(max_abs_distance);
    let expected = tick_pair(a_values.as_slice(), b_values.as_slice(), &metric, tier)?;
    Ok(json!({
        "name": name,
        "metric": "max_abs",
        "a": a,
        "b": b,
        "tier": tier_value(tier),
        "expected": expected,
    }))
}

fn custom_ladder_pair_case(
    name: &str,
    a: Value,
    b: Value,
    ladder_name: &str,
    ladder: &[Tier],
) -> Result<Value, Box<dyn std::error::Error>> {
    let a_values = number_array(&a);
    let b_values = number_array(&b);
    let metric = MetricFn(max_abs_distance);
    let expected = ladder_pair(a_values.as_slice(), b_values.as_slice(), &metric, ladder)?;
    Ok(json!({
        "name": name,
        "metric": "max_abs",
        "a": a,
        "b": b,
        "ladder": ladder_name,
        "expected": expected,
    }))
}

fn normalize_case(
    name: &str,
    mode: &str,
    input: &[f64],
) -> Result<Value, Box<dyn std::error::Error>> {
    let normalization = match mode {
        "none" => Normalization::None,
        "unit_max" => Normalization::UnitMax,
        "tanh" => Normalization::Tanh,
        _ => unreachable!("fixture generator only uses known normalizations"),
    };
    let mut out = vec![0.0; input.len()];
    normalize_ticks(input, normalization, &mut out)?;
    Ok(json!({
        "name": name,
        "mode": mode,
        "input": input,
        "expected": out,
    }))
}

fn carry_case(name: &str, epsilons: &[f64]) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!({
        "name": name,
        "epsilons": epsilons,
        "expected": carry_rules(epsilons)?,
    }))
}

fn promotion_case() -> Result<Value, Box<dyn std::error::Error>> {
    let quotas = vec![2, 3, 4];
    let mut counter = PromotionCounter::new(quotas.clone())?;
    let step_inputs = vec![
        Some(vec![false, false, false]),
        Some(vec![false, true, false]),
        None,
        None,
    ];
    let mut steps = Vec::new();
    for flags in step_inputs {
        let mut promoted = vec![false; counter.len()];
        counter.step(flags.as_deref(), &mut promoted)?;
        steps.push(json!({
            "event_flags": flags,
            "promoted": promoted,
            "counters": counter.counters(),
        }));
    }
    counter.reset();
    Ok(json!({
        "name": "explicit_quotas_with_reset",
        "quotas": quotas,
        "steps": steps,
        "after_reset_counters": counter.counters(),
    }))
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("fixture number")
}

fn number_array(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("fixture array")
        .iter()
        .map(number)
        .collect()
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
