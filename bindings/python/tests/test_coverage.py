"""CoverageMeter binding tests against the selected backend."""

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


def test_custom_callable_metric():
    # Chebyshev distinguishes itself from euclidean on ((0,0),(0.05,0.09)):
    # euclidean ~0.103 would admit at eps=0.1, chebyshev 0.09 must reject
    def chebyshev(a, b):
        return max(abs(x - y) for x, y in zip(a, b))

    with CoverageMeter([0.1], dim=2, metric=chebyshev) as meter:
        assert meter.observe([0.0, 0.0]) == [True]
        assert meter.observe([0.05, 0.09]) == [False]
        assert meter.counts == [1]
    with CoverageMeter([0.1], dim=2) as euclid:
        euclid.observe([0.0, 0.0])
        assert euclid.observe([0.05, 0.09]) == [True]


def test_raising_callable_metric_rejects_safely():
    def broken(_a, _b):
        raise ValueError("no distance")

    with CoverageMeter([0.1], dim=1, metric=broken) as meter:
        assert meter.observe([0.0]) == [True]  # first sample: empty store
        assert meter.observe([5.0]) == [False]  # NaN from the trampoline rejects
        assert meter.counts == [1]
