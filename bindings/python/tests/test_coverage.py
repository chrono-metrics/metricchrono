"""CoverageMeter binding tests against the native library."""

import math

import pytest

from metricchrono import (
    CoverageMeter,
    Metric,
    NativeStatusError,
    OperatingRegime,
    classify_regime,
    progress_efficiency,
)


def test_coverage_round_trip_and_pooling():
    with CoverageMeter([0.1, 0.2], dim=2) as meter:
        assert meter.tier_count == 2
        assert meter.observe([0.0, 0.0]) == [True, True]
        # 0.15 away: tier 0 admits (>= 0.1), tier 1 rejects (< 0.2)
        assert meter.observe([0.15, 0.0]) == [True, False]
        assert meter.counts == [2, 1]
        # two stored states despite three tier memberships
        assert meter.unique_representatives == 2


def test_creep_is_registered_with_zero_throughput():
    meter = CoverageMeter([0.1], dim=1, metric=Metric.ABSOLUTE)
    position = 0.0
    admitted_total = 0
    meter.observe([position])
    for _ in range(100):
        position += 0.05  # below epsilon: every per-step tick is silent
        admitted_total += sum(meter.observe([position]))
    assert meter.counts[0] > 30
    assert classify_regime(0.0, admitted_total) is OperatingRegime.CREEP


def test_quadrants_and_efficiency():
    assert classify_regime(0.0, 0) is OperatingRegime.QUIESCENT
    assert classify_regime(1.0, 1) is OperatingRegime.PROGRESS
    assert classify_regime(1.0, 0) is OperatingRegime.CHURN
    assert classify_regime(0.0, 1) is OperatingRegime.CREEP
    assert math.isclose(progress_efficiency(11, 0.1, 2.0), 0.5)
    with pytest.raises(NativeStatusError):
        progress_efficiency(11, 0.1, 0.0)


def test_invalid_construction_and_shape_errors():
    with pytest.raises(NativeStatusError):
        CoverageMeter([], dim=1)
    with pytest.raises(NativeStatusError):
        CoverageMeter([0.0], dim=1)
    with pytest.raises(NativeStatusError):
        CoverageMeter([0.1], dim=2, metric=Metric.ABSOLUTE)
    meter = CoverageMeter([0.1], dim=2)
    with pytest.raises(NativeStatusError):
        meter.observe([0.0])  # wrong dimension
    meter.close()
    with pytest.raises(RuntimeError):
        meter.observe([0.0, 0.0])
