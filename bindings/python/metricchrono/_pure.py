from __future__ import annotations

import math
import operator
import sys
from collections.abc import Callable, MutableSequence, Sequence
from typing import Any, Optional, Union

from .core import (
    Absolute,
    Euclidean,
    EventRecord,
    EventSummary,
    Ladder,
    MC_METRIC_ABSOLUTE,
    MC_METRIC_EUCLIDEAN,
    MC_NORMALIZATION_NONE,
    MC_NORMALIZATION_TANH,
    MC_NORMALIZATION_UNIT_MAX,
    MC_STATUS_INVALID_ARGUMENT,
    Metric,
    MetricFn,
    NativeStatusError,
    Normalization,
    OperatingRegime,
    Tier,
    ZoomDecision,
)

F64_MAX = sys.float_info.max
MAX_UINT64 = 2**64 - 1
MAX_PROMOTION_DEPTH = 1000

_OVERRIDES = [
    "_validated_tier",
    "tick_distance",
    "try_tick_distance",
    "euclidean_distance",
    "absolute_distance",
    "tick_pair",
    "ladder_distance",
    "ladder_values",
    "ladder_pair",
    "geometric_ladder",
    "custom_ladder",
    "validate_ladder",
    "normalize_ticks",
    "carry_rules",
    "smooth_tick_distance",
    "smooth_ladder_distance",
    "adaptive_ladder_distance",
    "weighted_consensus",
    "simple_weight_update",
    "PromotionCounter",
    "CoverageMeter",
    "progress_efficiency",
    "classify_regime",
    "EventLog",
]


def _invalid(message: str) -> NativeStatusError:
    return NativeStatusError(MC_STATUS_INVALID_ARGUMENT, message)


def _double_list(values: Sequence[float]) -> list[float]:
    return [float(value) for value in values]


def _ensure_uint64(value: int) -> None:
    if value < 0 or value > MAX_UINT64:
        raise ValueError("value must fit in uint64")


def _uint64_items(values: Sequence[int]) -> list[int]:
    items = [int(value) for value in values]
    for value in items:
        _ensure_uint64(value)
    return items


def _coerce_tiers(tiers: Union[Sequence[Tier], Ladder]) -> Sequence[Tier]:
    if isinstance(tiers, Ladder):
        return tiers.tiers
    return tiers


def _normalization_id(mode: Union[Normalization, str, int]) -> int:
    if isinstance(mode, Normalization):
        return int(mode)
    if isinstance(mode, str):
        normalized = mode.strip().lower().replace("-", "_")
        names = {
            "none": MC_NORMALIZATION_NONE,
            "unitmax": MC_NORMALIZATION_UNIT_MAX,
            "unit_max": MC_NORMALIZATION_UNIT_MAX,
            "tanh": MC_NORMALIZATION_TANH,
        }
        if normalized in names:
            return names[normalized]
        raise ValueError(f"unknown normalization mode: {mode}")
    return int(mode)


def _metric_id(metric: Any) -> Optional[int]:
    if isinstance(metric, Metric):
        return int(metric)
    if metric is Euclidean or metric is euclidean_distance:
        return MC_METRIC_EUCLIDEAN
    if metric is Absolute or metric is absolute_distance:
        return MC_METRIC_ABSOLUTE
    if isinstance(metric, str):
        normalized = metric.strip().lower().replace("-", "_")
        if normalized == "euclidean":
            return MC_METRIC_EUCLIDEAN
        if normalized == "absolute":
            return MC_METRIC_ABSOLUTE
        raise ValueError(f"unknown metric: {metric}")
    if isinstance(metric, int):
        return int(metric)
    return None


def _metric_distance(metric: Any, a: Any, b: Any) -> float:
    if isinstance(metric, MetricFn):
        return metric.distance(a, b)
    if callable(metric):
        return float(metric(a, b))
    raise TypeError("metric must be a Metric, MetricFn, metric name, id, or callable")


def _ensure_distance(distance: float) -> float:
    value = float(distance)
    if not math.isfinite(value) or value < 0.0:
        raise _invalid("distance must be finite and >= 0")
    return value


def _validate_tier(tier: Tier, index: int = 0) -> None:
    epsilon = float(tier.epsilon)
    delta = float(tier.delta)
    p = float(tier.p)
    epsilon_ref = float(tier.epsilon_ref)
    if not math.isfinite(epsilon) or epsilon <= 0.0:
        raise _invalid(f"invalid tier at index {index}: epsilon must be finite and > 0")
    if not math.isfinite(delta) or delta <= 0.0:
        raise _invalid(f"invalid tier at index {index}: delta must be finite and > 0")
    if epsilon >= delta:
        raise _invalid(f"invalid tier at index {index}: epsilon must be < delta")
    if not math.isfinite(p):
        raise _invalid(f"invalid tier at index {index}: p must be finite")
    if not math.isfinite(epsilon_ref) or epsilon_ref <= 0.0:
        raise _invalid(
            f"invalid tier at index {index}: epsilon_ref must be finite and > 0"
        )


def _validated_tier(epsilon: float, delta: float, p: float, epsilon_ref: float) -> Tier:
    value = Tier(float(epsilon), float(delta), float(p), float(epsilon_ref))
    _validate_tier(value)
    return value


def _sanitize_distance(distance: float) -> float:
    if math.isnan(distance) or math.copysign(1.0, distance) < 0.0:
        return 0.0
    if math.isinf(distance):
        return F64_MAX
    return distance


def _finite_or_max(value: float) -> float:
    if math.isnan(value):
        return 0.0
    if math.isinf(value):
        return F64_MAX
    return value


def _sanitize_signed(value: float) -> float:
    if math.isnan(value):
        return 0.0
    if value == math.inf:
        return F64_MAX
    if value == -math.inf:
        return -F64_MAX
    return value


def _clampf(value: float, lo: float, hi: float) -> float:
    if value < lo:
        return lo
    if value > hi:
        return hi
    return value


def _gain(tier: Tier) -> float:
    return math.pow(float(tier.epsilon) / float(tier.epsilon_ref), float(tier.p))


def _tick_distance_unchecked(distance: float, tier: Tier) -> float:
    d = _sanitize_distance(distance)
    if d < float(tier.epsilon):
        return 0.0
    return _finite_or_max(_gain(tier) * float(math.ceil(d / float(tier.delta))))


def tick_distance(distance: float, tier: Tier) -> float:
    _validate_tier(tier)
    d = _ensure_distance(distance)
    return _tick_distance_unchecked(d, tier)


def try_tick_distance(distance: float, tier: Tier) -> float:
    return tick_distance(distance, tier)


def euclidean_distance(a: Sequence[float], b: Sequence[float]) -> float:
    left = _double_list(a)
    right = _double_list(b)
    if len(left) != len(right):
        raise ValueError("a and b length must match")
    total = 0.0
    for lvalue, rvalue in zip(left, right):
        diff = lvalue - rvalue
        total += diff * diff
    return math.sqrt(total)


def absolute_distance(a: float, b: float) -> float:
    return abs(float(a) - float(b))


def tick_pair(a: Any, b: Any, metric: Any, tier: Tier) -> float:
    metric_id = _metric_id(metric)
    if metric_id is None:
        return tick_distance(_metric_distance(metric, a, b), tier)
    if metric_id == MC_METRIC_EUCLIDEAN:
        return tick_distance(euclidean_distance(a, b), tier)
    if metric_id == MC_METRIC_ABSOLUTE:
        return tick_distance(absolute_distance(a, b), tier)
    raise _invalid("unknown metric id")


def ladder_distance(distance: float, tiers: Union[Sequence[Tier], Ladder]) -> list[float]:
    values = list(_coerce_tiers(tiers))
    validate_ladder(values)
    d = _ensure_distance(distance)
    return [_tick_distance_unchecked(d, tier) for tier in values]


def ladder_values(distance: float, tiers: Union[Sequence[Tier], Ladder]) -> list[float]:
    return ladder_distance(distance, tiers)


def ladder_pair(
    a: Any,
    b: Any,
    metric: Any,
    tiers: Union[Sequence[Tier], Ladder],
) -> list[float]:
    metric_id = _metric_id(metric)
    if metric_id is None:
        return ladder_distance(_metric_distance(metric, a, b), tiers)
    if metric_id == MC_METRIC_EUCLIDEAN:
        return ladder_distance(euclidean_distance(a, b), tiers)
    if metric_id == MC_METRIC_ABSOLUTE:
        return ladder_distance(absolute_distance(a, b), tiers)
    raise _invalid("unknown metric id")


def _powi(base: float, n: int) -> float:
    if n < 0:
        return 1.0 / _powi(base, -n)
    result = 1.0
    bb = base
    e = n
    while e > 0:
        if e & 1:
            result = result * bb
        e >>= 1
        if e > 0:
            bb = bb * bb
    return result


def geometric_ladder(
    epsilon0: float,
    delta0: float,
    ratio: float,
    tiers: int,
    p: float = 0.5,
    epsilon_ref: float = 1.0,
) -> list[Tier]:
    count = operator.index(tiers)
    ratio_value = float(ratio)
    if count <= 0:
        raise _invalid("ladder must contain at least one tier")
    if not math.isfinite(ratio_value) or ratio_value <= 1.0:
        raise _invalid("ratio must be finite and > 1")
    out = []
    for k in range(count):
        scale = _powi(ratio_value, k)
        out.append(
            _validated_tier(
                float(epsilon0) * scale,
                float(delta0) * scale,
                float(p),
                float(epsilon_ref),
            )
        )
    validate_ladder(out)
    return out


def custom_ladder(tiers: Union[Sequence[Tier], Ladder]) -> list[Tier]:
    values = list(_coerce_tiers(tiers))
    validate_ladder(values)
    return list(values)


def validate_ladder(tiers: Union[Sequence[Tier], Ladder]) -> None:
    values = list(_coerce_tiers(tiers))
    if not values:
        raise _invalid("ladder must contain at least one tier")
    for index, tier in enumerate(values):
        _validate_tier(tier, index)
        if index > 0 and float(tier.epsilon) <= float(values[index - 1].epsilon):
            raise _invalid(
                f"invalid tier at index {index}: epsilon values must be strictly increasing"
            )
        if index > 0 and float(tier.delta) <= float(values[index - 1].delta):
            raise _invalid(
                f"invalid tier at index {index}: delta values must be strictly increasing"
            )


def normalize_ticks(
    ticks: Sequence[float],
    normalization: Union[Normalization, str, int] = Normalization.NONE,
) -> list[float]:
    values = _double_list(ticks)
    mode = _normalization_id(normalization)
    if mode == MC_NORMALIZATION_NONE:
        return list(values)
    if mode == MC_NORMALIZATION_UNIT_MAX:
        max_abs = 0.0
        for value in values:
            if math.isfinite(value):
                max_abs = max(max_abs, abs(value))
        if max_abs <= 0.0:
            return [0.0] * len(values)
        return [_sanitize_signed(value) / max_abs for value in values]
    if mode == MC_NORMALIZATION_TANH:
        return [math.tanh(_sanitize_signed(value)) for value in values]
    raise _invalid("unknown normalization mode")


def carry_rules(epsilons: Sequence[float]) -> list[int]:
    values = _double_list(epsilons)
    if not values:
        raise _invalid("ladder must contain at least one tier")
    out = []
    for index, epsilon in enumerate(values):
        if not math.isfinite(epsilon) or epsilon <= 0.0:
            raise _invalid(
                f"invalid tier at index {index}: epsilon must be finite and > 0"
            )
        quota = int(max(float(math.ceil(epsilon)), 1.0))
        out.append(min(quota, MAX_UINT64))
    return out


def _sigmoid(value: float) -> float:
    clipped = _clampf(value, -60.0, 60.0)
    return 1.0 / (1.0 + math.exp(-clipped))


def _smooth_stair(x: float, sharpness: float, max_stairs: int) -> float:
    if x <= 0.0:
        return _sigmoid(sharpness * x)
    hard = float(math.ceil(x))
    if not math.isfinite(hard) or hard > float(max_stairs):
        return hard
    j_max = int(hard) + 1
    total = 0.0
    for j in range(1, j_max + 1):
        total += _sigmoid(sharpness * (x - float(j)))
    return 1.0 + total


def _smooth_tick_distance_unchecked(
    distance: float,
    tier: Tier,
    gate_sharpness: float,
    stair_sharpness: float,
    max_stairs: int,
) -> float:
    d = distance
    gate = _sigmoid(gate_sharpness * (d - float(tier.epsilon)))
    x = d / float(tier.delta)
    stair = _smooth_stair(x, stair_sharpness, max_stairs)
    return _finite_or_max(_gain(tier) * gate * stair)


def smooth_tick_distance(distance: float, tier: Tier, sharpness: float) -> float:
    _validate_tier(tier)
    sharpness_value = float(sharpness)
    if not math.isfinite(sharpness_value) or sharpness_value <= 0.0:
        raise _invalid("sharpness must be finite and > 0")
    d = _ensure_distance(distance)
    return _smooth_tick_distance_unchecked(d, tier, sharpness_value, sharpness_value, 4096)


def smooth_ladder_distance(
    distance: float,
    tiers: Union[Sequence[Tier], Ladder],
    sharpness: float,
) -> list[float]:
    values = list(_coerce_tiers(tiers))
    validate_ladder(values)
    return [smooth_tick_distance(distance, tier, sharpness) for tier in values]


def adaptive_ladder_distance(
    distance: float,
    tiers: Union[Sequence[Tier], Ladder],
) -> tuple[list[float], ZoomDecision]:
    values = list(_coerce_tiers(tiers))
    validate_ladder(values)
    d = _ensure_distance(distance)
    ticks = [0.0] * len(values)
    for index, tier in enumerate(values):
        if d < float(tier.epsilon):
            return ticks, ZoomDecision(index + 1, index, True)
        ticks[index] = _tick_distance_unchecked(d, tier)
    return ticks, ZoomDecision(len(values), None, False)


def weighted_consensus(vectors: Sequence[Sequence[float]], weights: Sequence[float]) -> list[float]:
    if len(vectors) == 0:
        raise ValueError("vectors must not be empty")
    if len(vectors) != len(weights):
        raise ValueError("weights length must match vector count")
    cols = len(vectors[0])
    if cols == 0:
        raise ValueError("vectors must have at least one column")
    if any(len(row) != cols for row in vectors):
        raise ValueError("all vectors must have the same length")

    rows = [[float(value) for value in row] for row in vectors]
    weight_values = _double_list(weights)
    out = [0.0] * cols
    total_weight = 0.0
    for row, weight in zip(rows, weight_values):
        if not math.isfinite(weight) or weight < 0.0:
            raise _invalid("weights must be finite and >= 0")
        if weight == 0.0:
            continue
        total_weight += weight
        for index, value in enumerate(row):
            out[index] += weight * _sanitize_signed(value)
    if total_weight <= 0.0:
        raise _invalid("total consensus weight must be > 0")
    for index, value in enumerate(out):
        out[index] = value / total_weight
    return out


def simple_weight_update(
    weights: Union[MutableSequence[float], Sequence[float]],
    residuals: Sequence[float],
    learning_rate: float,
    floor: float,
) -> list[float]:
    if len(weights) != len(residuals):
        raise ValueError("weights and residuals length must match")
    values = _double_list(weights)
    residual_values = _double_list(residuals)
    if not values:
        raise _invalid("at least one weight is required")
    learning_rate_value = float(learning_rate)
    floor_value = float(floor)
    if not math.isfinite(learning_rate_value) or learning_rate_value < 0.0:
        raise _invalid("learning_rate must be finite and >= 0")
    if not math.isfinite(floor_value) or floor_value < 0.0:
        raise _invalid("floor must be finite and >= 0")

    total = 0.0
    updated = []
    for weight, residual in zip(values, residual_values):
        if (
            not math.isfinite(weight)
            or weight < 0.0
            or not math.isfinite(residual)
            or residual < 0.0
        ):
            raise _invalid("weights and residuals must be finite and >= 0")
        value = max(weight * math.exp(-learning_rate_value * residual), floor_value)
        updated.append(value)
        total += value
    if total <= 0.0:
        uniform = 1.0 / float(len(updated))
        updated = [uniform] * len(updated)
    else:
        for index, value in enumerate(updated):
            updated[index] = value / total
    if isinstance(weights, MutableSequence):
        weights[:] = updated
    return updated


class PromotionCounter:
    def __init__(self, quotas: Sequence[int]) -> None:
        quota_items = _uint64_items(quotas)
        if not quota_items:
            raise _invalid("ladder must contain at least one tier")
        if 0 in quota_items:
            raise _invalid("promotion quotas must be > 0")
        self._quotas = quota_items
        self._counters = [0] * len(quota_items)
        self._closed = False

    @classmethod
    def from_epsilons(cls, epsilons: Sequence[float]) -> "PromotionCounter":
        return cls(carry_rules(epsilons))

    @property
    def closed(self) -> bool:
        return self._closed

    @property
    def counters(self) -> list[int]:
        self._ensure_open()
        return list(self._counters)

    @property
    def quotas(self) -> list[int]:
        self._ensure_open()
        return list(self._quotas)

    def step(self, event_flags: Optional[Sequence[bool]] = None) -> list[bool]:
        self._ensure_open()
        flags = None
        if event_flags is not None:
            flags = [bool(value) for value in event_flags]
            if len(flags) != len(self._quotas):
                raise _invalid("shape mismatch for event flags")

        promoted = [False] * len(self._quotas)
        for index in range(len(self._counters)):
            event = flags[index] if flags is not None else False
            if not event:
                self._counters[index] = min(self._counters[index] + 1, MAX_UINT64)

        depth = 0
        while True:
            changed = False
            for index in range(len(self._quotas)):
                if self._counters[index] < self._quotas[index]:
                    continue
                self._counters[index] = 0
                promoted[index] = True
                if index + 1 < len(self._quotas):
                    self._counters[index + 1] = min(
                        self._counters[index + 1] + 1,
                        MAX_UINT64,
                    )
                changed = True
            if not changed:
                break
            depth += 1
            if depth > MAX_PROMOTION_DEPTH:
                raise _invalid("promotion depth exceeded")

        if flags is not None:
            for index, event in enumerate(flags):
                if event:
                    self._counters[index] = 0
        return promoted

    def reset(self) -> None:
        self._ensure_open()
        self._counters = [0] * len(self._quotas)

    def close(self) -> None:
        self._closed = True
        self._counters = []
        self._quotas = []

    def __enter__(self) -> "PromotionCounter":
        self._ensure_open()
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()

    def _ensure_open(self) -> None:
        if self._closed:
            raise RuntimeError("PromotionCounter is closed")


class CoverageMeter:
    """Per-tier streaming coverage meter (greedy maximal epsilon-packing)."""

    def __init__(
        self,
        epsilons: Sequence[float],
        dim: int,
        metric: Union[Metric, Callable[[list[float], list[float]], float]] = Metric.EUCLIDEAN,
    ) -> None:
        values = _double_list(epsilons)
        if not values:
            raise _invalid("ladder must contain at least one tier")
        if any(not math.isfinite(value) or value <= 0.0 for value in values):
            raise _invalid("coverage epsilons must be finite and positive")
        self._dim = operator.index(dim)
        if self._dim <= 0:
            raise _invalid("coverage state dimension must be > 0")
        self._epsilons = values
        self._pool: list[list[float]] = []
        self._tier_members: list[list[int]] = [[] for _ in values]
        self._closed = False

        if callable(metric) and not isinstance(metric, Metric):
            distance = metric

            def callback(left: list[float], right: list[float]) -> float:
                try:
                    return float(distance(left, right))
                except Exception:
                    return math.nan

            self._distance = callback
        else:
            metric_id = int(metric)
            if metric_id == MC_METRIC_ABSOLUTE:
                if self._dim != 1:
                    raise _invalid("absolute metric requires dimension 1")
                self._distance = lambda left, right: abs(left[0] - right[0])
            elif metric_id == MC_METRIC_EUCLIDEAN:
                self._distance = euclidean_distance
            else:
                raise _invalid("unknown metric id")

    @property
    def closed(self) -> bool:
        return self._closed

    @property
    def tier_count(self) -> int:
        self._ensure_open()
        return len(self._epsilons)

    @property
    def counts(self) -> list[int]:
        self._ensure_open()
        return [len(members) for members in self._tier_members]

    @property
    def unique_representatives(self) -> int:
        self._ensure_open()
        return len(self._pool)

    def observe(self, state: Sequence[float]) -> list[bool]:
        """Observe one sample; returns per-tier admission flags."""
        self._ensure_open()
        values = _double_list(state)
        if len(values) != self._dim:
            raise _invalid("shape mismatch for coverage state dimension")

        distances: dict[int, float] = {}

        def distance_to(index: int) -> float:
            if index not in distances:
                distances[index] = self._distance(self._pool[index], values)
            return distances[index]

        admitted = []
        for tier, epsilon in enumerate(self._epsilons):
            separated = True
            for index in reversed(self._tier_members[tier]):
                if not (distance_to(index) >= epsilon):
                    separated = False
                    break
            admitted.append(separated)

        if any(admitted):
            index = len(self._pool)
            self._pool.append(list(values))
            for tier, flag in enumerate(admitted):
                if flag:
                    self._tier_members[tier].append(index)
        return admitted

    def close(self) -> None:
        self._closed = True
        self._pool = []
        self._tier_members = []

    def __enter__(self) -> "CoverageMeter":
        self._ensure_open()
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()

    def _ensure_open(self) -> None:
        if self._closed:
            raise RuntimeError("CoverageMeter is closed")


def progress_efficiency(coverage: int, epsilon: float, path_length: float) -> float:
    """Fraction of traversed metric length that acquired new territory."""
    coverage_value = operator.index(coverage)
    _ensure_uint64(coverage_value)
    path = float(path_length)
    if not math.isfinite(path) or path <= 0.0:
        raise _invalid("path_length must be finite and positive")
    gained = max(coverage_value - 1, 0) * float(epsilon)
    return _clampf(gained / path, 0.0, 1.0)


def classify_regime(throughput_delta: float, coverage_delta: int) -> OperatingRegime:
    """Quadrant of a window: quiescent / progress / churn / creep."""
    coverage_value = operator.index(coverage_delta)
    _ensure_uint64(coverage_value)
    ticked = float(throughput_delta) > 0.0
    covered = coverage_value > 0
    if ticked and covered:
        return OperatingRegime.PROGRESS
    if ticked:
        return OperatingRegime.CHURN
    if covered:
        return OperatingRegime.CREEP
    return OperatingRegime.QUIESCENT


class EventLog:
    """Event skip-list."""

    def __init__(self, tier_count: int) -> None:
        count = operator.index(tier_count)
        if count < 0:
            raise ValueError("tier_count must be non-negative")
        if count == 0:
            raise _invalid("ladder must contain at least one tier")
        self._tier_count = count
        self._records: list[dict[str, Any]] = []
        self._first_by_tier: list[Optional[int]] = [None] * count
        self._last_by_tier: list[Optional[int]] = [None] * count
        self._closed = False

    @property
    def closed(self) -> bool:
        return self._closed

    def close(self) -> None:
        self._closed = True
        self._records = []
        self._first_by_tier = []
        self._last_by_tier = []

    def append(self, state_id: int, ticks: Sequence[float]) -> int:
        self._ensure_open()
        state_value = operator.index(state_id)
        _ensure_uint64(state_value)
        values = _double_list(ticks)
        if len(values) != self._tier_count:
            raise _invalid("shape mismatch for tick vector")

        index = len(self._records)
        record = {
            "state_id": state_value,
            "ticks": values,
            "next_event": [None] * self._tier_count,
        }
        self._records.append(record)
        for tier in range(self._tier_count):
            if _sanitize_signed(values[tier]) <= 0.0:
                continue
            previous = self._last_by_tier[tier]
            if previous is None:
                self._first_by_tier[tier] = index
            else:
                self._records[previous]["next_event"][tier] = index
            self._last_by_tier[tier] = index
        return index

    def first_event(self, tier: int) -> Optional[int]:
        self._ensure_open()
        tier_value = operator.index(tier)
        self._ensure_tier(tier_value)
        return self._first_by_tier[tier_value]

    def next_event(self, index: int, tier: int) -> Optional[int]:
        """Return the next event after an event record at index for tier."""
        self._ensure_open()
        index_value = operator.index(index)
        tier_value = operator.index(tier)
        self._ensure_index(index_value)
        self._ensure_tier(tier_value)
        return self._records[index_value]["next_event"][tier_value]

    def record(self, index: int) -> EventRecord:
        self._ensure_open()
        index_value = operator.index(index)
        self._ensure_index(index_value)
        record = self._records[index_value]
        return EventRecord(
            state_id=record["state_id"],
            ticks=list(record["ticks"]),
        )

    @property
    def records(self) -> list[EventRecord]:
        return [self.record(index) for index in range(len(self))]

    def compact_summary(self, tier: int) -> list[EventSummary]:
        self._ensure_open()
        tier_value = operator.index(tier)
        self._ensure_tier(tier_value)
        out = []
        index = self._first_by_tier[tier_value]
        while index is not None:
            record = self._records[index]
            out.append(
                EventSummary(
                    index=index,
                    state_id=record["state_id"],
                    tick=record["ticks"][tier_value],
                )
            )
            index = record["next_event"][tier_value]
        return out

    @property
    def tier_count(self) -> int:
        self._ensure_open()
        return self._tier_count

    @property
    def is_empty(self) -> bool:
        self._ensure_open()
        return len(self._records) == 0

    def __len__(self) -> int:
        self._ensure_open()
        return len(self._records)

    def __enter__(self) -> "EventLog":
        self._ensure_open()
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()

    def _ensure_open(self) -> None:
        if self._closed:
            raise RuntimeError("EventLog is closed")

    def _ensure_tier(self, tier: int) -> None:
        if tier < 0 or tier >= self._tier_count:
            raise _invalid("event log tier is out of bounds")

    def _ensure_index(self, index: int) -> None:
        if index < 0 or index >= len(self._records):
            raise _invalid("event log index is out of bounds")
