from __future__ import annotations

import csv
import json
import math
import struct
from pathlib import Path
from typing import Any

import metricchrono._pure as pure

ROOT = Path(__file__).resolve().parents[3] / "crates/metricchrono-core"
CONFORMANCE = ROOT / "fixtures/binding_conformance.v1.json"


class BitCounter:
    def __init__(self) -> None:
        self.passed = 0
        self.failed: list[str] = []

    def check_float(self, name: str, actual: float, expected: float) -> None:
        actual_bits = bits(actual)
        expected_bits = bits(expected)
        if actual_bits == expected_bits:
            self.passed += 1
            return
        self.failed.append(
            f"{name}: got {float(actual)!r} ({actual_bits.hex()}), "
            f"expected {float(expected)!r} ({expected_bits.hex()})"
        )

    def check_vector(self, name: str, actual: list[float], expected: list[float]) -> None:
        if len(actual) != len(expected):
            self.failed.append(f"{name}: length {len(actual)} != {len(expected)}")
            return
        for index, (left, right) in enumerate(zip(actual, expected)):
            self.check_float(f"{name}[{index}]", left, right)


def bits(value: float) -> bytes:
    return struct.pack("<d", float(value))


def tier_from_doc(document: dict[str, Any]) -> pure.Tier:
    return pure.Tier(
        float(document["epsilon"]),
        float(document["delta"]),
        float(document["p"]),
        float(document["epsilon_ref"]),
    )


def max_abs_distance(a: list[float], b: list[float]) -> float:
    if len(a) != len(b):
        return math.nan
    return max(abs(left - right) for left, right in zip(a, b))


def metric_from_name(name: str) -> Any:
    if name == "euclidean":
        return pure.Euclidean
    if name == "absolute":
        return pure.Absolute
    if name == "max_abs":
        return pure.MetricFn(max_abs_distance)
    raise AssertionError(f"unknown metric {name}")


def build_ladders(fixture: dict[str, Any], counter: BitCounter) -> dict[str, list[pure.Tier]]:
    ladders: dict[str, list[pure.Tier]] = {}
    for case in fixture["ladders"]:
        expected = [tier_from_doc(tier) for tier in case["tiers"]]
        if case["kind"] == "geometric":
            params = case["params"]
            actual = pure.geometric_ladder(
                params["epsilon0"],
                params["delta0"],
                params["ratio"],
                params["tiers"],
                params["p"],
                params["epsilon_ref"],
            )
            for index, (left, right) in enumerate(zip(actual, expected)):
                prefix = f"ladder/{case['name']}/construct[{index}]"
                counter.check_float(f"{prefix}.epsilon", left.epsilon, right.epsilon)
                counter.check_float(f"{prefix}.delta", left.delta, right.delta)
                counter.check_float(f"{prefix}.p", left.p, right.p)
                counter.check_float(
                    f"{prefix}.epsilon_ref",
                    left.epsilon_ref,
                    right.epsilon_ref,
                )
        elif case["kind"] == "custom":
            actual = pure.custom_ladder(expected)
        else:
            raise AssertionError(f"unknown ladder kind {case['kind']}")

        pure.validate_ladder(actual)
        for distance_case in case["distances"]:
            counter.check_vector(
                f"ladder/{case['name']}/distance={distance_case['distance']}",
                pure.ladder_distance(distance_case["distance"], actual),
                distance_case["expected"],
            )
        ladders[case["name"]] = actual
    return ladders


def test_pure_backend_byte_identity(capsys: Any) -> None:
    fixture = json.loads(CONFORMANCE.read_text())
    counter = BitCounter()
    ladders = build_ladders(fixture, counter)

    for case in fixture["tick_distance_cases"]:
        counter.check_float(
            f"tick/{case['name']}",
            pure.tick_distance(case["distance"], tier_from_doc(case["tier"])),
            case["expected"],
        )

    for case in fixture["smooth_tick_distance_cases"]:
        counter.check_float(
            f"smooth_tick/{case['name']}",
            pure.smooth_tick_distance(
                case["distance"],
                tier_from_doc(case["tier"]),
                case["sharpness"],
            ),
            case["expected"],
        )

    for case in fixture["smooth_ladder_distance_cases"]:
        counter.check_vector(
            f"smooth_ladder/{case['name']}",
            pure.smooth_ladder_distance(
                case["distance"],
                ladders[case["ladder"]],
                case["sharpness"],
            ),
            case["expected"],
        )

    for case in fixture["adaptive_ladder_distance_cases"]:
        ticks, decision = pure.adaptive_ladder_distance(
            case["distance"],
            ladders[case["ladder"]],
        )
        counter.check_vector(f"adaptive/{case['name']}", ticks, case["expected"]["ticks"])
        assert decision.evaluated_tiers == case["expected"]["decision"]["evaluated_tiers"]
        assert decision.first_inactive_tier == case["expected"]["decision"]["first_inactive_tier"]
        assert decision.stopped_early == case["expected"]["decision"]["stopped_early"]

    for case in fixture["weighted_consensus_cases"]:
        counter.check_vector(
            f"weighted_consensus/{case['name']}",
            pure.weighted_consensus(case["vectors"], case["weights"]),
            case["expected"],
        )

    for case in fixture["metric_cases"]["absolute_distance"]:
        counter.check_float(
            f"absolute_distance/{case['name']}",
            pure.absolute_distance(case["a"], case["b"]),
            case["expected"],
        )

    for case in fixture["metric_cases"]["euclidean_distance"]:
        counter.check_float(
            f"euclidean_distance/{case['name']}",
            pure.euclidean_distance(case["a"], case["b"]),
            case["expected"],
        )

    for case in fixture["pair_cases"]["tick_pair"]:
        counter.check_float(
            f"tick_pair/{case['name']}",
            pure.tick_pair(
                case["a"],
                case["b"],
                metric_from_name(case["metric"]),
                tier_from_doc(case["tier"]),
            ),
            case["expected"],
        )

    for case in fixture["pair_cases"]["ladder_pair"]:
        counter.check_vector(
            f"ladder_pair/{case['name']}",
            pure.ladder_pair(
                case["a"],
                case["b"],
                metric_from_name(case["metric"]),
                ladders[case["ladder"]],
            ),
            case["expected"],
        )

    for case in fixture["normalize_ticks_cases"]:
        counter.check_vector(
            f"normalize/{case['name']}",
            pure.normalize_ticks(case["input"], case["mode"]),
            case["expected"],
        )

    with (ROOT / "fixtures/golden_ticks.csv").open(newline="") as handle:
        for row in csv.DictReader(handle):
            tier = pure.Tier(
                float(row["epsilon"]),
                float(row["delta"]),
                float(row["p"]),
                float(row["epsilon_ref"]),
            )
            counter.check_float(
                f"golden_ticks/{row['name']}",
                pure.tick_distance(float(row["distance"]), tier),
                float(row["expected"]),
            )

    with (ROOT / "fixtures/golden_ladders.csv").open(newline="") as handle:
        for row in csv.DictReader(handle):
            ladder = pure.geometric_ladder(
                float(row["epsilon0"]),
                float(row["delta0"]),
                float(row["ratio"]),
                int(row["tiers"]),
                float(row["p"]),
                float(row["epsilon_ref"]),
            )
            counter.check_vector(
                f"golden_ladders/{row['name']}",
                pure.ladder_distance(float(row["distance"]), ladder),
                [float(value) for value in row["expected"].split(";")],
            )

    with capsys.disabled():
        print(f"pure_byte_identity: PASS {counter.passed} FAIL {len(counter.failed)}")
    assert not counter.failed, "\n".join(counter.failed)
