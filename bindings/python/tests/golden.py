from __future__ import annotations

import csv
import json
from pathlib import Path

import metricchrono as mc

ROOT = Path(__file__).resolve().parents[3] / "crates/metricchrono-core"
EPS = 1e-12


def assert_close(name: str, actual: float, expected: float) -> None:
    if abs(actual - expected) > EPS:
        raise AssertionError(f"{name}: expected {expected}, got {actual}")


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
            if len(actual) != len(expected):
                raise AssertionError(f"{row['name']}: length mismatch")
            for index, (left, right) in enumerate(zip(actual, expected)):
                assert_close(f"{row['name']}[{index}]", left, right)


def test_public_api_surface() -> None:
    ladder = mc.Ladder.geometric(0.5, 1.0, 2.0, 4, 0.5, 1.0)
    assert len(ladder) == 4
    values = mc.ladder_distance(3.0, ladder)
    assert len(values) == 4
    assert values[0] > values[1] > values[2] > values[3]
    assert ladder.values(3.0) == values

    smooth = mc.smooth_ladder_distance(3.0, ladder, 10.0)
    assert len(smooth) == 4
    assert all(value > 0.0 for value in smooth)

    early, decision = mc.adaptive_ladder_distance(0.75, ladder)
    assert early[0] > 0.0
    assert early[1:] == [0.0, 0.0, 0.0]
    assert decision.first_inactive_tier == 1
    assert decision.stopped_early

    consensus = mc.weighted_consensus([[1.0, 2.0], [3.0, 0.0]], [0.25, 0.75])
    assert consensus == [2.5, 0.5]
    residuals = mc.coherence_residuals([[1.0, 2.0], [3.0, 0.0]], consensus)
    weights = [0.5, 0.5]
    updated = mc.simple_weight_update(weights, residuals, 0.2, 0.01)
    assert weights == updated
    assert_close("updated weights sum", sum(updated), 1.0)

    with mc.EventLog(2) as log:
        assert log.append(10, [1.0, 0.0]) == 0
        assert log.append(11, [1.0, 1.0]) == 1
        assert len(log) == 2
        assert log.next_event(0, 0) == 1
        assert log.next_event(0, 1) is None

    try:
        mc.tick_distance(-1.0, mc.Tier(0.5, 1.0, 0.0, 1.0))
    except mc.NativeStatusError:
        pass
    else:
        raise AssertionError("negative distance should raise")

    try:
        mc.tick_distance(1.0, mc.Tier(1.0, 1.0, 0.0, 1.0))
    except mc.NativeStatusError:
        pass
    else:
        raise AssertionError("epsilon >= delta should raise")


def test_schema_round_trip() -> None:
    with (ROOT.parents[1] / "tests/golden/ladder.v1.json").open() as handle:
        ladder_doc = json.load(handle)
    ladder = mc.ladder_from_schema(ladder_doc)
    assert mc.ladder_distance(1.0, ladder) == [10.0, 4.0, 2.0]
    assert mc.ladder_to_schema(ladder) == ladder_doc

    with (ROOT.parents[1] / "tests/golden/tick_vector.v1.json").open() as handle:
        tick_doc = json.load(handle)
    ticks = mc.tick_vector_from_schema(tick_doc)
    assert ticks == [10.0, 4.0, 2.0]
    assert mc.tick_vector_to_schema(ticks) == tick_doc

    with (ROOT.parents[1] / "tests/golden/consensus_result.v1.json").open() as handle:
        consensus_doc = json.load(handle)
    assert mc.consensus_result_from_schema(consensus_doc) == consensus_doc


if __name__ == "__main__":
    test_ticks()
    test_ladders()
    test_public_api_surface()
    test_schema_round_trip()
