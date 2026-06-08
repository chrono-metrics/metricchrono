from __future__ import annotations

import ctypes
import math
import os
import sys
from collections.abc import Callable, Iterable, Iterator, MutableSequence, Sequence
from dataclasses import dataclass
from enum import IntEnum
from functools import lru_cache
from pathlib import Path
from typing import Any, Mapping, Optional, Union

MC_STATUS_OK = 0
MC_STATUS_NULL = 1
MC_STATUS_INVALID_ARGUMENT = 2
MC_STATUS_BUFFER_TOO_SMALL = 3
MC_STATUS_PANIC = 255

MC_METRIC_EUCLIDEAN = 0
MC_METRIC_ABSOLUTE = 1

MC_NORMALIZATION_NONE = 0
MC_NORMALIZATION_UNIT_MAX = 1
MC_NORMALIZATION_TANH = 2


class MetricChronoError(RuntimeError):
    """Base class for MetricChrono Python binding errors."""


class NativeLoadError(MetricChronoError):
    """Raised when the MetricChrono shared library cannot be loaded."""


class NativeStatusError(MetricChronoError):
    """Raised when the MetricChrono C ABI returns an error status."""

    def __init__(self, status: int, message: Optional[str] = None) -> None:
        self.status = status
        self.message = message
        names = {
            MC_STATUS_NULL: "null pointer",
            MC_STATUS_INVALID_ARGUMENT: "invalid argument",
            MC_STATUS_BUFFER_TOO_SMALL: "buffer too small",
            MC_STATUS_PANIC: "panic",
        }
        super().__init__(message or names.get(status, f"unknown status {status}"))


class Metric(IntEnum):
    EUCLIDEAN = MC_METRIC_EUCLIDEAN
    ABSOLUTE = MC_METRIC_ABSOLUTE


Euclidean = Metric.EUCLIDEAN
Absolute = Metric.ABSOLUTE


class Normalization(IntEnum):
    NONE = MC_NORMALIZATION_NONE
    UNIT_MAX = MC_NORMALIZATION_UNIT_MAX
    TANH = MC_NORMALIZATION_TANH


class MetricFn:
    """Function-backed metric escape hatch for tick_pair and ladder_pair."""

    def __init__(self, func: Callable[[Any, Any], float]) -> None:
        self._func = func

    def distance(self, a: Any, b: Any) -> float:
        return float(self._func(a, b))

    def __call__(self, a: Any, b: Any) -> float:
        return self.distance(a, b)


@dataclass(frozen=True)
class Tier:
    epsilon: float
    delta: float
    p: float = 0.5
    epsilon_ref: float = 1.0

    @classmethod
    def builder(cls) -> "TierBuilder":
        return TierBuilder()


class TierBuilder:
    def __init__(
        self,
        epsilon: float = 1.0,
        delta: float = 2.0,
        p: float = 0.5,
        epsilon_ref: float = 1.0,
    ) -> None:
        self._epsilon = float(epsilon)
        self._delta = float(delta)
        self._p = float(p)
        self._epsilon_ref = float(epsilon_ref)

    def epsilon(self, value: float) -> "TierBuilder":
        return TierBuilder(value, self._delta, self._p, self._epsilon_ref)

    def delta(self, value: float) -> "TierBuilder":
        return TierBuilder(self._epsilon, value, self._p, self._epsilon_ref)

    def p(self, value: float) -> "TierBuilder":
        return TierBuilder(self._epsilon, self._delta, value, self._epsilon_ref)

    def epsilon_ref(self, value: float) -> "TierBuilder":
        return TierBuilder(self._epsilon, self._delta, self._p, value)

    def build(self) -> Tier:
        return _validated_tier(self._epsilon, self._delta, self._p, self._epsilon_ref)


@dataclass(frozen=True)
class Ladder:
    tiers: tuple[Tier, ...]

    def __init__(self, tiers: Sequence[Tier]) -> None:
        object.__setattr__(self, "tiers", tuple(custom_ladder(tiers)))

    @classmethod
    def geometric(
        cls,
        epsilon0: float,
        delta0: float,
        ratio: float,
        tiers: int,
        p: float = 0.5,
        epsilon_ref: float = 1.0,
    ) -> "Ladder":
        return cls(geometric_ladder(epsilon0, delta0, ratio, tiers, p, epsilon_ref))

    def values(self, distance: float) -> list[float]:
        return ladder_distance(distance, self)

    def __iter__(self) -> Iterator[Tier]:
        return iter(self.tiers)

    def __len__(self) -> int:
        return len(self.tiers)

    def __getitem__(self, index: int) -> Tier:
        return self.tiers[index]


@dataclass(frozen=True)
class ZoomDecision:
    evaluated_tiers: int
    first_inactive_tier: Optional[int]
    stopped_early: bool


@dataclass(frozen=True)
class EventRecord:
    state_id: int
    ticks: list[float]


@dataclass(frozen=True)
class EventSummary:
    index: int
    state_id: int
    tick: float


class _MCTier(ctypes.Structure):
    _fields_ = [
        ("epsilon", ctypes.c_double),
        ("delta", ctypes.c_double),
        ("p", ctypes.c_double),
        ("epsilon_ref", ctypes.c_double),
    ]


class _MCZoomDecision(ctypes.Structure):
    _fields_ = [
        ("evaluated_tiers", ctypes.c_size_t),
        ("first_inactive_tier", ctypes.c_size_t),
        ("has_first_inactive_tier", ctypes.c_bool),
        ("stopped_early", ctypes.c_bool),
    ]


def _library_filename() -> str:
    if sys.platform == "win32":
        return "metricchrono_ffi.dll"
    if sys.platform == "darwin":
        return "libmetricchrono_ffi.dylib"
    return "libmetricchrono_ffi.so"


def _candidate_paths() -> list[Path]:
    filename = _library_filename()
    candidates: list[Path] = []
    env_path = os.environ.get("METRICCHRONO_FFI_LIB")
    if env_path:
        candidates.append(Path(env_path).expanduser())

    here = Path(__file__).resolve()
    for parent in [here.parent, *here.parents]:
        candidates.extend(
            [
                parent / filename,
                parent / "target" / "release" / filename,
                parent / "target" / "debug" / filename,
            ]
        )
    return candidates


@lru_cache(maxsize=1)
def _load_library() -> ctypes.CDLL:
    attempted: list[str] = []
    for path in _candidate_paths():
        attempted.append(str(path))
        if path.exists():
            try:
                lib = ctypes.CDLL(str(path))
                _configure_library(lib)
                return lib
            except OSError:
                continue
    tried = ", ".join(attempted)
    raise NativeLoadError(
        "metricchrono_ffi shared library was not found. Build it with "
        "`cargo build -p metricchrono-ffi --release` or set METRICCHRONO_FFI_LIB. "
        f"Tried: {tried}"
    )


def _configure_library(lib: ctypes.CDLL) -> None:
    lib.mc_error_message.argtypes = [ctypes.c_int]
    lib.mc_error_message.restype = ctypes.c_char_p

    lib.mc_last_error_message.argtypes = [
        ctypes.POINTER(ctypes.c_char),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_last_error_message.restype = ctypes.c_int

    lib.mc_tier_new.argtypes = [
        ctypes.c_double,
        ctypes.c_double,
        ctypes.c_double,
        ctypes.c_double,
        ctypes.POINTER(_MCTier),
    ]
    lib.mc_tier_new.restype = ctypes.c_int

    lib.mc_ladder_new.argtypes = [
        ctypes.POINTER(_MCTier),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.mc_ladder_new.restype = ctypes.c_int

    lib.mc_custom_ladder.argtypes = [
        ctypes.POINTER(_MCTier),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.mc_custom_ladder.restype = ctypes.c_int

    lib.mc_ladder_free.argtypes = [ctypes.c_void_p]
    lib.mc_ladder_free.restype = None

    lib.mc_ladder_len.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t)]
    lib.mc_ladder_len.restype = ctypes.c_int

    lib.mc_validate_ladder.argtypes = [ctypes.c_void_p]
    lib.mc_validate_ladder.restype = ctypes.c_int

    lib.mc_ladder_distance_owned.argtypes = [
        ctypes.c_void_p,
        ctypes.c_double,
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
    ]
    lib.mc_ladder_distance_owned.restype = ctypes.c_int

    lib.mc_tick_distance.argtypes = [
        ctypes.c_double,
        _MCTier,
        ctypes.POINTER(ctypes.c_double),
    ]
    lib.mc_tick_distance.restype = ctypes.c_int

    lib.mc_euclidean_distance.argtypes = [
        ctypes.POINTER(ctypes.c_double),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_double),
    ]
    lib.mc_euclidean_distance.restype = ctypes.c_int

    lib.mc_absolute_distance.argtypes = [
        ctypes.POINTER(ctypes.c_double),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_double),
    ]
    lib.mc_absolute_distance.restype = ctypes.c_int

    lib.mc_tick_pair.argtypes = [
        ctypes.c_int,
        ctypes.POINTER(ctypes.c_double),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        _MCTier,
        ctypes.POINTER(ctypes.c_double),
    ]
    lib.mc_tick_pair.restype = ctypes.c_int

    lib.mc_ladder_distance.argtypes = [
        ctypes.c_double,
        ctypes.POINTER(_MCTier),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
    ]
    lib.mc_ladder_distance.restype = ctypes.c_int

    lib.mc_ladder_pair.argtypes = [
        ctypes.c_int,
        ctypes.POINTER(ctypes.c_double),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_ladder_pair.restype = ctypes.c_int

    lib.mc_adaptive_ladder_distance.argtypes = [
        ctypes.c_double,
        ctypes.POINTER(_MCTier),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(_MCZoomDecision),
    ]
    lib.mc_adaptive_ladder_distance.restype = ctypes.c_int

    lib.mc_smooth_tick_distance.argtypes = [
        ctypes.c_double,
        _MCTier,
        ctypes.c_double,
        ctypes.POINTER(ctypes.c_double),
    ]
    lib.mc_smooth_tick_distance.restype = ctypes.c_int

    lib.mc_geometric_ladder.argtypes = [
        ctypes.c_double,
        ctypes.c_double,
        ctypes.c_double,
        ctypes.c_size_t,
        ctypes.c_double,
        ctypes.c_double,
        ctypes.POINTER(_MCTier),
        ctypes.c_size_t,
    ]
    lib.mc_geometric_ladder.restype = ctypes.c_int

    lib.mc_normalize_ticks.argtypes = [
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.c_int,
        ctypes.POINTER(ctypes.c_double),
    ]
    lib.mc_normalize_ticks.restype = ctypes.c_int

    lib.mc_carry_rules.argtypes = [
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_uint64),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_carry_rules.restype = ctypes.c_int

    lib.mc_weighted_consensus.argtypes = [
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_double),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
    ]
    lib.mc_weighted_consensus.restype = ctypes.c_int

    lib.mc_simple_weight_update.argtypes = [
        ctypes.POINTER(ctypes.c_double),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.c_double,
        ctypes.c_double,
    ]
    lib.mc_simple_weight_update.restype = ctypes.c_int

    lib.mc_promotion_counter_new.argtypes = [
        ctypes.POINTER(ctypes.c_uint64),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.mc_promotion_counter_new.restype = ctypes.c_int

    lib.mc_promotion_counter_from_epsilons.argtypes = [
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.mc_promotion_counter_from_epsilons.restype = ctypes.c_int

    lib.mc_promotion_counter_step.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_bool),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_bool),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_promotion_counter_step.restype = ctypes.c_int

    lib.mc_promotion_counter_counters.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_uint64),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_promotion_counter_counters.restype = ctypes.c_int

    lib.mc_promotion_counter_quotas.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_uint64),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_promotion_counter_quotas.restype = ctypes.c_int

    lib.mc_promotion_counter_reset.argtypes = [ctypes.c_void_p]
    lib.mc_promotion_counter_reset.restype = ctypes.c_int

    lib.mc_promotion_counter_free.argtypes = [ctypes.c_void_p]
    lib.mc_promotion_counter_free.restype = None

    lib.mc_event_log_new.argtypes = [ctypes.c_size_t]
    lib.mc_event_log_new.restype = ctypes.c_void_p
    lib.mc_event_log_free.argtypes = [ctypes.c_void_p]
    lib.mc_event_log_free.restype = None
    lib.mc_event_log_append.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint64,
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_event_log_append.restype = ctypes.c_int
    lib.mc_event_log_first_event.argtypes = [
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
        ctypes.POINTER(ctypes.c_bool),
    ]
    lib.mc_event_log_first_event.restype = ctypes.c_int
    lib.mc_event_log_next_event.argtypes = [
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
        ctypes.POINTER(ctypes.c_bool),
    ]
    lib.mc_event_log_next_event.restype = ctypes.c_int
    lib.mc_event_log_record.argtypes = [
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_uint64),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_event_log_record.restype = ctypes.c_int
    lib.mc_event_log_compact_summary.argtypes = [
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
        ctypes.POINTER(ctypes.c_uint64),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_event_log_compact_summary.restype = ctypes.c_int
    lib.mc_event_log_len.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t)]
    lib.mc_event_log_len.restype = ctypes.c_int
    lib.mc_event_log_tier_count.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.mc_event_log_tier_count.restype = ctypes.c_int
    lib.mc_event_log_is_empty.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_bool),
    ]
    lib.mc_event_log_is_empty.restype = ctypes.c_int


def _tier(value: Tier) -> _MCTier:
    return _MCTier(value.epsilon, value.delta, value.p, value.epsilon_ref)


def _from_tier(value: _MCTier) -> Tier:
    return Tier(value.epsilon, value.delta, value.p, value.epsilon_ref)


def _tier_array(values: Sequence[Tier]) -> Any:
    return (_MCTier * len(values))(*[_tier(value) for value in values])


def _double_list(values: Iterable[float]) -> list[float]:
    return [float(value) for value in values]


def _double_array(values: Iterable[float]) -> Any:
    items = _double_list(values)
    return (ctypes.c_double * len(items))(*items)


def _uint64_items(values: Iterable[int]) -> list[int]:
    items = [int(value) for value in values]
    for value in items:
        _ensure_uint64(value)
    return items


def _uint64_array(values: Iterable[int]) -> Any:
    items = _uint64_items(values)
    return (ctypes.c_uint64 * len(items))(*items)


def _bool_array(values: Iterable[bool]) -> Any:
    items = [bool(value) for value in values]
    return (ctypes.c_bool * len(items))(*items)


def _ensure_uint64(value: int) -> None:
    if value < 0 or value > 2**64 - 1:
        raise ValueError("value must fit in uint64")


def _check(status: int) -> None:
    if status != MC_STATUS_OK:
        raise NativeStatusError(status, _last_error_message(status))


def _check_sizing_status(status: int) -> None:
    if status not in (MC_STATUS_OK, MC_STATUS_BUFFER_TOO_SMALL):
        _check(status)


def _last_error_message(status: int) -> str:
    lib = _load_library()
    out_len = ctypes.c_size_t()
    first = lib.mc_last_error_message(None, 0, ctypes.byref(out_len))
    if first not in (MC_STATUS_OK, MC_STATUS_BUFFER_TOO_SMALL):
        return _status_message(lib, status)

    needed = int(out_len.value)
    if needed <= 0:
        return _status_message(lib, status)

    buf = ctypes.create_string_buffer(needed)
    second = lib.mc_last_error_message(buf, needed, ctypes.byref(out_len))
    if second != MC_STATUS_OK:
        return _status_message(lib, status)

    message = buf.value.decode("utf-8", errors="replace")
    return message or _status_message(lib, status)


def _status_message(lib: ctypes.CDLL, status: int) -> str:
    raw = lib.mc_error_message(status)
    if raw:
        return raw.decode("utf-8", errors="replace")
    return f"unknown status {status}"


def _validated_tier(epsilon: float, delta: float, p: float, epsilon_ref: float) -> Tier:
    out = _MCTier()
    status = _load_library().mc_tier_new(
        float(epsilon),
        float(delta),
        float(p),
        float(epsilon_ref),
        ctypes.byref(out),
    )
    _check(status)
    return _from_tier(out)


def _create_ladder_handle(tiers: Union[Sequence[Tier], Ladder]) -> ctypes.c_void_p:
    values = _coerce_tiers(tiers)
    tier_array = _tier_array(values)
    out = ctypes.c_void_p()
    status = _load_library().mc_custom_ladder(tier_array, len(values), ctypes.byref(out))
    _check(status)
    if not out.value:
        raise NativeStatusError(MC_STATUS_INVALID_ARGUMENT)
    return out


def _free_ladder_handle(handle: ctypes.c_void_p) -> None:
    if handle.value:
        _load_library().mc_ladder_free(handle)


def _double_pair(a: Iterable[float], b: Iterable[float]) -> tuple[Any, Any, int]:
    left = _double_list(a)
    right = _double_list(b)
    if len(left) != len(right):
        raise ValueError("a and b length must match")
    return _double_array(left), _double_array(right), len(left)


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


def _native_metric_pair(metric_id: int, a: Any, b: Any) -> tuple[Any, Any, int]:
    if metric_id == MC_METRIC_EUCLIDEAN:
        return _double_pair(a, b)
    if metric_id == MC_METRIC_ABSOLUTE:
        return _double_array([float(a)]), _double_array([float(b)]), 1
    return _double_array([]), _double_array([]), 0


def _metric_distance(metric: Any, a: Any, b: Any) -> float:
    if isinstance(metric, MetricFn):
        return metric.distance(a, b)
    if callable(metric):
        return float(metric(a, b))
    raise TypeError("metric must be a Metric, MetricFn, metric name, id, or callable")


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


def tick_distance(distance: float, tier: Tier) -> float:
    out = ctypes.c_double()
    status = _load_library().mc_tick_distance(float(distance), _tier(tier), ctypes.byref(out))
    _check(status)
    return float(out.value)


def try_tick_distance(distance: float, tier: Tier) -> float:
    return tick_distance(distance, tier)


def euclidean_distance(a: Sequence[float], b: Sequence[float]) -> float:
    a_array, b_array, length = _double_pair(a, b)
    out = ctypes.c_double()
    status = _load_library().mc_euclidean_distance(
        a_array,
        b_array,
        length,
        ctypes.byref(out),
    )
    _check(status)
    return float(out.value)


def absolute_distance(a: float, b: float) -> float:
    a_array = _double_array([float(a)])
    b_array = _double_array([float(b)])
    out = ctypes.c_double()
    status = _load_library().mc_absolute_distance(
        a_array,
        b_array,
        1,
        ctypes.byref(out),
    )
    _check(status)
    return float(out.value)


def tick_pair(a: Any, b: Any, metric: Any, tier: Tier) -> float:
    metric_id = _metric_id(metric)
    if metric_id is None:
        return tick_distance(_metric_distance(metric, a, b), tier)

    a_array, b_array, length = _native_metric_pair(metric_id, a, b)
    out = ctypes.c_double()
    status = _load_library().mc_tick_pair(
        metric_id,
        a_array,
        b_array,
        length,
        _tier(tier),
        ctypes.byref(out),
    )
    _check(status)
    return float(out.value)


def ladder_distance(distance: float, tiers: Union[Sequence[Tier], Ladder]) -> list[float]:
    values = _coerce_tiers(tiers)
    tier_array = _tier_array(values)
    out = (ctypes.c_double * len(values))()
    status = _load_library().mc_ladder_distance(
        float(distance),
        tier_array,
        len(values),
        out,
        len(values),
    )
    _check(status)
    return [float(value) for value in out]


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

    a_array, b_array, length = _native_metric_pair(metric_id, a, b)
    handle = _create_ladder_handle(tiers)
    try:
        out_len = ctypes.c_size_t()
        status = _load_library().mc_ladder_pair(
            metric_id,
            a_array,
            b_array,
            length,
            handle,
            None,
            0,
            ctypes.byref(out_len),
        )
        _check_sizing_status(status)

        needed = int(out_len.value)
        out = (ctypes.c_double * needed)()
        status = _load_library().mc_ladder_pair(
            metric_id,
            a_array,
            b_array,
            length,
            handle,
            out,
            needed,
            ctypes.byref(out_len),
        )
        _check(status)
        return [float(out[index]) for index in range(int(out_len.value))]
    finally:
        _free_ladder_handle(handle)


def geometric_ladder(
    epsilon0: float,
    delta0: float,
    ratio: float,
    tiers: int,
    p: float = 0.5,
    epsilon_ref: float = 1.0,
) -> list[Tier]:
    out = (_MCTier * tiers)()
    status = _load_library().mc_geometric_ladder(
        float(epsilon0),
        float(delta0),
        float(ratio),
        tiers,
        float(p),
        float(epsilon_ref),
        out,
        tiers,
    )
    _check(status)
    return [_from_tier(value) for value in out]


def custom_ladder(tiers: Union[Sequence[Tier], Ladder]) -> list[Tier]:
    values = list(_coerce_tiers(tiers))
    handle = _create_ladder_handle(values)
    try:
        return list(values)
    finally:
        _free_ladder_handle(handle)


def validate_ladder(tiers: Union[Sequence[Tier], Ladder]) -> None:
    handle = _create_ladder_handle(tiers)
    try:
        status = _load_library().mc_validate_ladder(handle)
        _check(status)
    finally:
        _free_ladder_handle(handle)


def tier_from_schema(document: Mapping[str, Any]) -> Tier:
    _ensure_schema(
        document,
        "tier.v1",
        ["metricchrono_schema", "epsilon", "delta", "p", "epsilon_ref"],
    )
    return _validated_tier(
        float(document["epsilon"]),
        float(document["delta"]),
        float(document["p"]),
        float(document["epsilon_ref"]),
    )


def tier_to_schema(value: Tier) -> dict[str, Union[float, str]]:
    validated = _validated_tier(value.epsilon, value.delta, value.p, value.epsilon_ref)
    return {
        "metricchrono_schema": "tier.v1",
        "epsilon": validated.epsilon,
        "delta": validated.delta,
        "p": validated.p,
        "epsilon_ref": validated.epsilon_ref,
    }


def ladder_from_schema(document: Mapping[str, Any]) -> list[Tier]:
    _ensure_schema(document, "ladder.v1", ["metricchrono_schema", "tiers"])
    tiers = []
    for index, item in enumerate(document["tiers"]):
        _ensure_exact_fields(
            item,
            ["epsilon", "delta", "p", "epsilon_ref"],
            f"tier at index {index}",
        )
        tiers.append(
            Tier(
                float(item["epsilon"]),
                float(item["delta"]),
                float(item["p"]),
                float(item["epsilon_ref"]),
            )
        )
    return custom_ladder(tiers)


def ladder_to_schema(tiers: Union[Sequence[Tier], Ladder]) -> dict[str, Any]:
    values = custom_ladder(tiers)
    return {
        "metricchrono_schema": "ladder.v1",
        "tiers": [
            {
                "epsilon": tier.epsilon,
                "delta": tier.delta,
                "p": tier.p,
                "epsilon_ref": tier.epsilon_ref,
            }
            for tier in values
        ],
    }


def tick_vector_from_schema(document: Mapping[str, Any]) -> list[float]:
    _ensure_schema(document, "tick_vector.v1", ["metricchrono_schema", "ticks"])
    return [float(value) for value in document["ticks"]]


def tick_vector_to_schema(ticks: Sequence[float]) -> dict[str, Any]:
    return {
        "metricchrono_schema": "tick_vector.v1",
        "ticks": [float(value) for value in ticks],
    }


def consensus_result_from_schema(document: Mapping[str, Any]) -> dict[str, Union[list[float], str]]:
    _ensure_schema(
        document,
        "consensus_result.v1",
        ["metricchrono_schema", "consensus", "residuals", "weights"],
    )
    return {
        "metricchrono_schema": "consensus_result.v1",
        "consensus": [float(value) for value in document["consensus"]],
        "residuals": [float(value) for value in document["residuals"]],
        "weights": [float(value) for value in document["weights"]],
    }


def normalize_ticks(
    ticks: Sequence[float],
    normalization: Union[Normalization, str, int] = Normalization.NONE,
) -> list[float]:
    values = _double_list(ticks)
    tick_array = _double_array(values)
    out = (ctypes.c_double * len(values))()
    status = _load_library().mc_normalize_ticks(
        tick_array,
        len(values),
        _normalization_id(normalization),
        out,
    )
    _check(status)
    return [float(value) for value in out]


def carry_rules(epsilons: Sequence[float]) -> list[int]:
    values = _double_list(epsilons)
    epsilon_array = _double_array(values)
    out_len = ctypes.c_size_t()
    status = _load_library().mc_carry_rules(
        epsilon_array,
        len(values),
        None,
        0,
        ctypes.byref(out_len),
    )
    _check_sizing_status(status)

    needed = int(out_len.value)
    out = (ctypes.c_uint64 * needed)()
    status = _load_library().mc_carry_rules(
        epsilon_array,
        len(values),
        out,
        needed,
        ctypes.byref(out_len),
    )
    _check(status)
    return [int(out[index]) for index in range(int(out_len.value))]


def smooth_tick_distance(distance: float, tier: Tier, sharpness: float) -> float:
    out = ctypes.c_double()
    status = _load_library().mc_smooth_tick_distance(
        float(distance),
        _tier(tier),
        float(sharpness),
        ctypes.byref(out),
    )
    _check(status)
    return float(out.value)


def smooth_ladder_distance(
    distance: float,
    tiers: Union[Sequence[Tier], Ladder],
    sharpness: float,
) -> list[float]:
    return [smooth_tick_distance(distance, tier, sharpness) for tier in _coerce_tiers(tiers)]


def adaptive_ladder_distance(
    distance: float,
    tiers: Union[Sequence[Tier], Ladder],
) -> tuple[list[float], ZoomDecision]:
    values = _coerce_tiers(tiers)
    tier_array = _tier_array(values)
    out = (ctypes.c_double * len(values))()
    raw_decision = _MCZoomDecision()
    status = _load_library().mc_adaptive_ladder_distance(
        float(distance),
        tier_array,
        len(values),
        out,
        len(values),
        ctypes.byref(raw_decision),
    )
    _check(status)
    decision = ZoomDecision(
        evaluated_tiers=int(raw_decision.evaluated_tiers),
        first_inactive_tier=(
            int(raw_decision.first_inactive_tier)
            if raw_decision.has_first_inactive_tier
            else None
        ),
        stopped_early=bool(raw_decision.stopped_early),
    )
    return [float(value) for value in out], decision


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
    flat = [float(value) for row in vectors for value in row]
    flat_array = _double_array(flat)
    weight_array = _double_array(weights)
    out = (ctypes.c_double * cols)()
    status = _load_library().mc_weighted_consensus(
        flat_array,
        len(vectors),
        cols,
        weight_array,
        out,
        cols,
    )
    _check(status)
    return [float(value) for value in out]


def coherence_residual(source_tick: Sequence[float], consensus: Sequence[float]) -> float:
    if len(source_tick) != len(consensus):
        raise ValueError("source_tick and consensus length must match")
    if not consensus:
        raise ValueError("consensus must not be empty")
    mse = sum(
        (_sanitize_signed(left) - _sanitize_signed(right)) ** 2
        for left, right in zip(source_tick, consensus)
    ) / len(consensus)
    return math.sqrt(mse)


def coherence_residuals(
    vectors: Sequence[Sequence[float]],
    consensus: Sequence[float],
) -> list[float]:
    return [coherence_residual(vector, consensus) for vector in vectors]


def simple_weight_update(
    weights: Union[MutableSequence[float], Sequence[float]],
    residuals: Sequence[float],
    learning_rate: float,
    floor: float,
) -> list[float]:
    if len(weights) != len(residuals):
        raise ValueError("weights and residuals length must match")
    weight_array = _double_array(weights)
    residual_array = _double_array(residuals)
    status = _load_library().mc_simple_weight_update(
        weight_array,
        residual_array,
        len(weights),
        float(learning_rate),
        float(floor),
    )
    _check(status)
    updated = [float(value) for value in weight_array]
    if isinstance(weights, MutableSequence):
        weights[:] = updated
    return updated


class PromotionCounter:
    def __init__(self, quotas: Sequence[int]) -> None:
        quota_items = _uint64_items(quotas)
        quota_array = (ctypes.c_uint64 * len(quota_items))(*quota_items)
        out = ctypes.c_void_p()
        status = _load_library().mc_promotion_counter_new(
            quota_array,
            len(quota_items),
            ctypes.byref(out),
        )
        _check(status)
        if not out.value:
            raise NativeStatusError(MC_STATUS_INVALID_ARGUMENT)
        self._ptr = out.value

    @classmethod
    def from_epsilons(cls, epsilons: Sequence[float]) -> "PromotionCounter":
        values = _double_list(epsilons)
        epsilon_array = _double_array(values)
        out = ctypes.c_void_p()
        status = _load_library().mc_promotion_counter_from_epsilons(
            epsilon_array,
            len(values),
            ctypes.byref(out),
        )
        _check(status)
        if not out.value:
            raise NativeStatusError(MC_STATUS_INVALID_ARGUMENT)
        counter = cls.__new__(cls)
        counter._ptr = out.value
        return counter

    @property
    def closed(self) -> bool:
        return self._ptr is None

    @property
    def counters(self) -> list[int]:
        self._ensure_open()
        return self._read_uint64_vector("mc_promotion_counter_counters")

    @property
    def quotas(self) -> list[int]:
        self._ensure_open()
        return self._read_uint64_vector("mc_promotion_counter_quotas")

    def step(self, event_flags: Optional[Sequence[bool]] = None) -> list[bool]:
        self._ensure_open()
        flags_array = None
        flags_len = 0
        if event_flags is not None:
            flags_array = _bool_array(event_flags)
            flags_len = len(event_flags)

        out_len = ctypes.c_size_t()
        status = _load_library().mc_promotion_counter_step(
            self._ptr,
            None,
            0,
            None,
            0,
            ctypes.byref(out_len),
        )
        _check_sizing_status(status)

        needed = int(out_len.value)
        out = (ctypes.c_bool * needed)()
        status = _load_library().mc_promotion_counter_step(
            self._ptr,
            flags_array,
            flags_len,
            out,
            needed,
            ctypes.byref(out_len),
        )
        _check(status)
        return [bool(out[index]) for index in range(int(out_len.value))]

    def reset(self) -> None:
        self._ensure_open()
        status = _load_library().mc_promotion_counter_reset(self._ptr)
        _check(status)

    def close(self) -> None:
        ptr = getattr(self, "_ptr", None)
        if ptr is not None:
            _load_library().mc_promotion_counter_free(ptr)
            self._ptr = None

    def __enter__(self) -> "PromotionCounter":
        self._ensure_open()
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()

    def _read_uint64_vector(self, name: str) -> list[int]:
        native = getattr(_load_library(), name)
        out_len = ctypes.c_size_t()
        status = native(self._ptr, None, 0, ctypes.byref(out_len))
        _check_sizing_status(status)

        needed = int(out_len.value)
        out = (ctypes.c_uint64 * needed)()
        status = native(self._ptr, out, needed, ctypes.byref(out_len))
        _check(status)
        return [int(out[index]) for index in range(int(out_len.value))]

    def _ensure_open(self) -> None:
        if getattr(self, "_ptr", None) is None:
            raise RuntimeError("PromotionCounter is closed")


class EventLog:
    """Event skip-list.

    next_event follows from an existing event record; first_event is the
    chain head for a tier.
    """

    def __init__(self, tier_count: int) -> None:
        if tier_count < 0:
            raise ValueError("tier_count must be non-negative")
        self._ptr = _load_library().mc_event_log_new(tier_count)
        if not self._ptr:
            raise NativeStatusError(
                MC_STATUS_INVALID_ARGUMENT,
                _last_error_message(MC_STATUS_INVALID_ARGUMENT),
            )

    @property
    def closed(self) -> bool:
        return self._ptr is None

    def close(self) -> None:
        if self._ptr is not None:
            _load_library().mc_event_log_free(self._ptr)
            self._ptr = None

    def append(self, state_id: int, ticks: Sequence[float]) -> int:
        self._ensure_open()
        _ensure_uint64(state_id)
        tick_array = _double_array(ticks)
        out = ctypes.c_size_t()
        status = _load_library().mc_event_log_append(
            self._ptr,
            state_id,
            tick_array,
            len(ticks),
            ctypes.byref(out),
        )
        _check(status)
        return int(out.value)

    def first_event(self, tier: int) -> Optional[int]:
        self._ensure_open()
        out = ctypes.c_size_t()
        has_event = ctypes.c_bool()
        status = _load_library().mc_event_log_first_event(
            self._ptr,
            tier,
            ctypes.byref(out),
            ctypes.byref(has_event),
        )
        _check(status)
        return int(out.value) if has_event.value else None

    def next_event(self, index: int, tier: int) -> Optional[int]:
        """Return the next event after an event record at index for tier."""
        self._ensure_open()
        out = ctypes.c_size_t()
        has_event = ctypes.c_bool()
        status = _load_library().mc_event_log_next_event(
            self._ptr,
            index,
            tier,
            ctypes.byref(out),
            ctypes.byref(has_event),
        )
        _check(status)
        return int(out.value) if has_event.value else None

    def record(self, index: int) -> EventRecord:
        self._ensure_open()
        state_id = ctypes.c_uint64()
        out_len = ctypes.c_size_t()
        status = _load_library().mc_event_log_record(
            self._ptr,
            index,
            ctypes.byref(state_id),
            None,
            0,
            ctypes.byref(out_len),
        )
        _check_sizing_status(status)

        needed = int(out_len.value)
        ticks = (ctypes.c_double * needed)()
        status = _load_library().mc_event_log_record(
            self._ptr,
            index,
            ctypes.byref(state_id),
            ticks,
            needed,
            ctypes.byref(out_len),
        )
        _check(status)
        return EventRecord(
            state_id=int(state_id.value),
            ticks=[float(ticks[offset]) for offset in range(int(out_len.value))],
        )

    @property
    def records(self) -> list[EventRecord]:
        return [self.record(index) for index in range(len(self))]

    def compact_summary(self, tier: int) -> list[EventSummary]:
        self._ensure_open()
        out_len = ctypes.c_size_t()
        status = _load_library().mc_event_log_compact_summary(
            self._ptr,
            tier,
            None,
            None,
            None,
            0,
            ctypes.byref(out_len),
        )
        _check_sizing_status(status)

        needed = int(out_len.value)
        idx_out = (ctypes.c_size_t * needed)()
        state_out = (ctypes.c_uint64 * needed)()
        tick_out = (ctypes.c_double * needed)()
        status = _load_library().mc_event_log_compact_summary(
            self._ptr,
            tier,
            idx_out,
            state_out,
            tick_out,
            needed,
            ctypes.byref(out_len),
        )
        _check(status)
        return [
            EventSummary(
                index=int(idx_out[offset]),
                state_id=int(state_out[offset]),
                tick=float(tick_out[offset]),
            )
            for offset in range(int(out_len.value))
        ]

    @property
    def tier_count(self) -> int:
        self._ensure_open()
        out = ctypes.c_size_t()
        status = _load_library().mc_event_log_tier_count(self._ptr, ctypes.byref(out))
        _check(status)
        return int(out.value)

    @property
    def is_empty(self) -> bool:
        self._ensure_open()
        out = ctypes.c_bool()
        status = _load_library().mc_event_log_is_empty(self._ptr, ctypes.byref(out))
        _check(status)
        return bool(out.value)

    def __len__(self) -> int:
        self._ensure_open()
        out = ctypes.c_size_t()
        status = _load_library().mc_event_log_len(self._ptr, ctypes.byref(out))
        _check(status)
        return int(out.value)

    def __enter__(self) -> "EventLog":
        self._ensure_open()
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()

    def _ensure_open(self) -> None:
        if self._ptr is None:
            raise RuntimeError("EventLog is closed")


def _sanitize_signed(value: float) -> float:
    if math.isnan(value):
        return 0.0
    if value == math.inf:
        return sys.float_info.max
    if value == -math.inf:
        return -sys.float_info.max
    return float(value)


def _coerce_tiers(tiers: Union[Sequence[Tier], Ladder]) -> Sequence[Tier]:
    if isinstance(tiers, Ladder):
        return tiers.tiers
    return tiers


def _ensure_schema(
    document: Mapping[str, Any],
    expected: str,
    fields: Sequence[str],
) -> None:
    _ensure_exact_fields(document, fields, expected)
    if document.get("metricchrono_schema") != expected:
        raise ValueError(f"expected schema {expected}")


def _ensure_exact_fields(
    document: Mapping[str, Any],
    fields: Sequence[str],
    context: str,
) -> None:
    expected = set(fields)
    actual = set(document.keys())
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        parts = []
        if missing:
            parts.append(f"missing fields: {', '.join(missing)}")
        if extra:
            parts.append(f"unknown fields: {', '.join(extra)}")
        raise ValueError(f"{context} schema fields mismatch ({'; '.join(parts)})")
