from __future__ import annotations

import csv
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


if __name__ == "__main__":
    test_ticks()
    test_ladders()
