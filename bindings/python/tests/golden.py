from __future__ import annotations

import csv
import json
import math
from pathlib import Path
from typing import Any

import metricchrono as mc

ROOT = Path(__file__).resolve().parents[3] / "crates/metricchrono-core"
REPO_ROOT = ROOT.parents[1]
CONFORMANCE = ROOT / "fixtures/binding_conformance.v1.json"
EPS = 1e-12


def assert_close(name: str, actual: float, expected: float, eps: float = EPS) -> None:
    if abs(actual - expected) > eps:
        raise AssertionError(f"{name}: expected {expected}, got {actual}")


def assert_vector_close(
    name: str,
    actual: list[float] | tuple[float, ...],
    expected: list[float],
    eps: float = EPS,
) -> None:
    if len(actual) != len(expected):
        raise AssertionError(f"{name}: length mismatch, expected {len(expected)}, got {len(actual)}")
    for index, (left, right) in enumerate(zip(actual, expected)):
        assert_close(f"{name}[{index}]", float(left), float(right), eps)


def load_conformance() -> dict[str, Any]:
    with CONFORMANCE.open() as handle:
        return json.load(handle)


def tier_from_doc(document: dict[str, Any]) -> mc.Tier:
    return mc.Tier(
        float(document["epsilon"]),
        float(document["delta"]),
        float(document["p"]),
        float(document["epsilon_ref"]),
    )


def ladder_map(fixture: dict[str, Any]) -> dict[str, list[mc.Tier]]:
    ladders: dict[str, list[mc.Tier]] = {}
    for case in fixture["ladders"]:
        expected = [tier_from_doc(tier) for tier in case["tiers"]]
        if case["kind"] == "geometric":
            params = case["params"]
            actual = mc.geometric_ladder(
                params["epsilon0"],
                params["delta0"],
                params["ratio"],
                params["tiers"],
                params["p"],
                params["epsilon_ref"],
            )
            object_ladder = mc.Ladder.geometric(
                params["epsilon0"],
                params["delta0"],
                params["ratio"],
                params["tiers"],
                params["p"],
                params["epsilon_ref"],
            )
        elif case["kind"] == "custom":
            actual = mc.custom_ladder(expected)
            object_ladder = mc.Ladder(expected)
        else:
            raise AssertionError(f"unknown ladder kind {case['kind']}")

        mc.validate_ladder(actual)
        if len(actual) != len(expected):
            raise AssertionError(f"{case['name']}: tier length mismatch")
        for index, (left, right) in enumerate(zip(actual, expected)):
            assert_close(f"{case['name']}[{index}].epsilon", left.epsilon, right.epsilon)
            assert_close(f"{case['name']}[{index}].delta", left.delta, right.delta)
            assert_close(f"{case['name']}[{index}].p", left.p, right.p)
            assert_close(f"{case['name']}[{index}].epsilon_ref", left.epsilon_ref, right.epsilon_ref)

        for distance_case in case["distances"]:
            distance = float(distance_case["distance"])
            expected_values = [float(value) for value in distance_case["expected"]]
            assert_vector_close(
                f"{case['name']}/ladder_distance",
                mc.ladder_distance(distance, actual),
                expected_values,
            )
            assert_vector_close(
                f"{case['name']}/ladder_values",
                mc.ladder_values(distance, actual),
                expected_values,
            )
            assert_vector_close(
                f"{case['name']}/Ladder.values",
                object_ladder.values(distance),
                expected_values,
            )
        ladders[case["name"]] = actual
    return ladders


def test_ticks() -> None:
    with (ROOT / "fixtures/golden_ticks.csv").open(newline="") as handle:
        for row in csv.DictReader(handle):
            tier = mc.Tier(
                float(row["epsilon"]),
                float(row["delta"]),
                float(row["p"]),
                float(row["epsilon_ref"]),
            )
            actual = mc.tick_distance(float(row["distance"]), tier)
            assert_close(row["name"], actual, float(row["expected"]))


def test_ladders() -> None:
    with (ROOT / "fixtures/golden_ladders.csv").open(newline="") as handle:
        for row in csv.DictReader(handle):
            tiers = []
            epsilon0 = float(row["epsilon0"])
            delta0 = float(row["delta0"])
            ratio = float(row["ratio"])
            for index in range(int(row["tiers"])):
                scale = ratio**index
                tiers.append(
                    mc.Tier(
                        epsilon0 * scale,
                        delta0 * scale,
                        float(row["p"]),
                        float(row["epsilon_ref"]),
                    )
                )
            actual = mc.ladder_distance(float(row["distance"]), tiers)
            expected = [float(value) for value in row["expected"].split(";")]
            assert_vector_close(row["name"], actual, expected)


def test_binding_conformance_fixture() -> None:
    fixture = load_conformance()
    if fixture["metricchrono_schema"] != "binding_conformance.v1":
        raise AssertionError("unexpected conformance fixture schema")
    tolerance = float(fixture["tolerance"])
    ladders = ladder_map(fixture)

    for case in fixture["tick_distance_cases"]:
        tier = tier_from_doc(case["tier"])
        expected = float(case["expected"])
        assert_close(case["name"], mc.tick_distance(case["distance"], tier), expected, tolerance)
        assert_close(
            f"{case['name']}/try_tick_distance",
            mc.try_tick_distance(case["distance"], tier),
            expected,
            tolerance,
        )

    for case in fixture["smooth_tick_distance_cases"]:
        actual = mc.smooth_tick_distance(
            case["distance"],
            tier_from_doc(case["tier"]),
            case["sharpness"],
        )
        assert_close(case["name"], actual, case["expected"], tolerance)

    for case in fixture["smooth_ladder_distance_cases"]:
        actual = mc.smooth_ladder_distance(
            case["distance"],
            ladders[case["ladder"]],
            case["sharpness"],
        )
        assert_vector_close(case["name"], actual, case["expected"], tolerance)

    for case in fixture["adaptive_ladder_distance_cases"]:
        ticks, decision = mc.adaptive_ladder_distance(case["distance"], ladders[case["ladder"]])
        expected = case["expected"]
        assert_vector_close(case["name"], ticks, expected["ticks"], tolerance)
        assert decision.evaluated_tiers == expected["decision"]["evaluated_tiers"]
        assert decision.first_inactive_tier == expected["decision"]["first_inactive_tier"]
        assert decision.stopped_early == expected["decision"]["stopped_early"]

    for case in fixture["weighted_consensus_cases"]:
        actual = mc.weighted_consensus(case["vectors"], case["weights"])
        assert_vector_close(case["name"], actual, case["expected"], tolerance)

    for case in fixture["event_log_cases"]:
        assert_event_log_case(case, tolerance)

    assert_metric_cases(fixture["metric_cases"], tolerance)
    assert_pair_cases(fixture["pair_cases"], ladders, tolerance)

    for case in fixture["normalize_ticks_cases"]:
        actual = mc.normalize_ticks(case["input"], case["mode"])
        assert_vector_close(case["name"], actual, case["expected"], tolerance)

    for case in fixture["carry_rules_cases"]:
        actual = mc.carry_rules(case["epsilons"])
        if actual != case["expected"]:
            raise AssertionError(f"{case['name']}: expected {case['expected']}, got {actual}")

    for case in fixture["promotion_counter_cases"]:
        with mc.PromotionCounter(case["quotas"]) as counter:
            if counter.quotas != case["quotas"]:
                raise AssertionError(f"{case['name']}: quotas mismatch")
            for step in case["steps"]:
                actual = counter.step(step["event_flags"])
                if actual != step["promoted"]:
                    raise AssertionError(f"{case['name']}: promoted mismatch")
                if counter.counters != step["counters"]:
                    raise AssertionError(f"{case['name']}: counters mismatch")
            counter.reset()
            if counter.counters != case["after_reset_counters"]:
                raise AssertionError(f"{case['name']}: reset counters mismatch")

    assert_rejections(fixture["rejections"], fixture["event_log_cases"][0])


def assert_event_log_case(case: dict[str, Any], tolerance: float) -> None:
    with mc.EventLog(case["tier_count"]) as log:
        assert log.is_empty == case["is_empty_before_append"]
        for record in case["append_records"]:
            actual_index = log.append(record["state_id"], record["ticks"])
            if actual_index != record["expected_index"]:
                raise AssertionError(f"{case['name']}: append index mismatch")

        assert len(log) == case["expected_len"]
        assert log.is_empty == case["is_empty_after_append"]
        assert log.tier_count == case["tier_count"]

        for expected in case["records"]:
            actual = log.record(expected["index"])
            if actual.state_id != expected["state_id"]:
                raise AssertionError(f"{case['name']}: state id mismatch")
            assert_vector_close(
                f"{case['name']}/record[{expected['index']}]",
                actual.ticks,
                expected["ticks"],
                tolerance,
            )
        if len(log.records) != case["expected_len"]:
            raise AssertionError(f"{case['name']}: records property length mismatch")

        for expected in case["first_events"]:
            assert log.first_event(expected["tier"]) == expected["expected"]

        for expected in case["next_events"]:
            assert log.next_event(expected["index"], expected["tier"]) == expected["expected"]

        for summary in case["compact_summaries"]:
            actual = log.compact_summary(summary["tier"])
            expected_items = summary["expected"]
            if len(actual) != len(expected_items):
                raise AssertionError(f"{case['name']}: compact summary length mismatch")
            for left, right in zip(actual, expected_items):
                assert left.index == right["index"]
                assert left.state_id == right["state_id"]
                assert_close("compact summary tick", left.tick, right["tick"], tolerance)


def assert_metric_cases(cases: dict[str, Any], tolerance: float) -> None:
    for case in cases["euclidean_distance"]:
        assert_close(
            case["name"],
            mc.euclidean_distance(case["a"], case["b"]),
            case["expected"],
            tolerance,
        )
    for case in cases["absolute_distance"]:
        assert_close(
            case["name"],
            mc.absolute_distance(case["a"], case["b"]),
            case["expected"],
            tolerance,
        )


def assert_pair_cases(
    cases: dict[str, Any],
    ladders: dict[str, list[mc.Tier]],
    tolerance: float,
) -> None:
    for case in cases["tick_pair"]:
        metric = metric_from_name(case["metric"])
        actual = mc.tick_pair(case["a"], case["b"], metric, tier_from_doc(case["tier"]))
        assert_close(case["name"], actual, case["expected"], tolerance)

    for case in cases["ladder_pair"]:
        metric = metric_from_name(case["metric"])
        actual = mc.ladder_pair(case["a"], case["b"], metric, ladders[case["ladder"]])
        assert_vector_close(case["name"], actual, case["expected"], tolerance)


def metric_from_name(name: str) -> Any:
    if name == "euclidean":
        return mc.Euclidean
    if name == "absolute":
        return mc.Absolute
    if name == "max_abs":
        return mc.MetricFn(max_abs_distance)
    raise AssertionError(f"unknown metric {name}")


def max_abs_distance(a: list[float], b: list[float]) -> float:
    if len(a) != len(b):
        return math.nan
    return max(abs(left - right) for left, right in zip(a, b))


def assert_rejections(rejections: dict[str, Any], event_case: dict[str, Any]) -> None:
    for case in rejections["invalid_tiers"]:
        expect_raises(
            case["name"],
            lambda case=case: mc.Tier.builder()
            .epsilon(case["epsilon"])
            .delta(case["delta"])
            .p(case["p"])
            .epsilon_ref(case["epsilon_ref"])
            .build(),
        )

    for case in rejections["unknown_schema_documents"]:
        if case["kind"] == "tier":
            expect_raises(case["name"], lambda case=case: mc.tier_from_schema(case["document"]))
        else:
            raise AssertionError(f"unknown schema rejection kind {case['kind']}")

    with mc.EventLog(event_case["tier_count"]) as log:
        for record in event_case["append_records"]:
            log.append(record["state_id"], record["ticks"])
        for case in rejections["event_log_out_of_range"]:
            operation = case["operation"]
            if operation == "record":
                expect_raises(case["name"], lambda case=case: log.record(case["index"]))
            elif operation == "next_event":
                expect_raises(
                    case["name"],
                    lambda case=case: log.next_event(case["index"], case["tier"]),
                )
            elif operation == "first_event":
                expect_raises(case["name"], lambda case=case: log.first_event(case["tier"]))
            elif operation == "compact_summary":
                expect_raises(case["name"], lambda case=case: log.compact_summary(case["tier"]))
            else:
                raise AssertionError(f"unknown event log rejection operation {operation}")


def expect_raises(name: str, func: Any) -> None:
    try:
        func()
    except Exception:
        return
    raise AssertionError(f"{name}: expected rejection")


def test_schema_round_trip() -> None:
    with (REPO_ROOT / "tests/golden/ladder.v1.json").open() as handle:
        ladder_doc = json.load(handle)
    ladder = mc.ladder_from_schema(ladder_doc)
    assert mc.ladder_distance(1.0, ladder) == [10.0, 4.0, 2.0]
    assert mc.ladder_to_schema(ladder) == ladder_doc

    with (REPO_ROOT / "tests/golden/tick_vector.v1.json").open() as handle:
        tick_doc = json.load(handle)
    ticks = mc.tick_vector_from_schema(tick_doc)
    assert ticks == [10.0, 4.0, 2.0]
    assert mc.tick_vector_to_schema(ticks) == tick_doc

    with (REPO_ROOT / "tests/golden/consensus_result.v1.json").open() as handle:
        consensus_doc = json.load(handle)
    assert mc.consensus_result_from_schema(consensus_doc) == consensus_doc


if __name__ == "__main__":
    test_ticks()
    test_ladders()
    test_binding_conformance_fixture()
    test_schema_round_trip()
