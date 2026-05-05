from __future__ import annotations

import ctypes
import math
import os
import sys
from collections.abc import Iterable, MutableSequence, Sequence
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Any, Optional, Union

MC_STATUS_OK = 0
MC_STATUS_NULL = 1
MC_STATUS_INVALID_ARGUMENT = 2
MC_STATUS_BUFFER_TOO_SMALL = 3
MC_STATUS_PANIC = 255


class NativeLoadError(RuntimeError):
    """Raised when the MetricChrono shared library cannot be loaded."""


class NativeStatusError(RuntimeError):
    """Raised when the MetricChrono C ABI returns an error status."""

    def __init__(self, status: int) -> None:
        self.status = status
        names = {
            MC_STATUS_NULL: "null pointer",
            MC_STATUS_INVALID_ARGUMENT: "invalid argument",
            MC_STATUS_BUFFER_TOO_SMALL: "buffer too small",
            MC_STATUS_PANIC: "panic",
        }
        super().__init__(names.get(status, f"unknown status {status}"))


@dataclass(frozen=True)
class Tier:
    epsilon: float
    delta: float
    p: float = 0.5
    epsilon_ref: float = 1.0


@dataclass(frozen=True)
class ZoomDecision:
    evaluated_tiers: int
    first_inactive_tier: Optional[int]
    stopped_early: bool


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
    lib.mc_tick_distance.argtypes = [
        ctypes.c_double,
        _MCTier,
        ctypes.POINTER(ctypes.c_double),
    ]
    lib.mc_tick_distance.restype = ctypes.c_int

    lib.mc_ladder_distance.argtypes = [
        ctypes.c_double,
        ctypes.POINTER(_MCTier),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
    ]
    lib.mc_ladder_distance.restype = ctypes.c_int

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
    lib.mc_event_log_next_event.argtypes = [
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_size_t),
        ctypes.POINTER(ctypes.c_bool),
    ]
    lib.mc_event_log_next_event.restype = ctypes.c_int
    lib.mc_event_log_len.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t)]
    lib.mc_event_log_len.restype = ctypes.c_int


def _tier(value: Tier) -> _MCTier:
    return _MCTier(value.epsilon, value.delta, value.p, value.epsilon_ref)


def _from_tier(value: _MCTier) -> Tier:
    return Tier(value.epsilon, value.delta, value.p, value.epsilon_ref)


def _tier_array(values: Sequence[Tier]) -> Any:
    return (_MCTier * len(values))(*[_tier(value) for value in values])


def _double_array(values: Iterable[float]) -> Any:
    items = [float(value) for value in values]
    return (ctypes.c_double * len(items))(*items)


def _check(status: int) -> None:
    if status != MC_STATUS_OK:
        raise NativeStatusError(status)


def tick_distance(distance: float, tier: Tier) -> float:
    out = ctypes.c_double()
    status = _load_library().mc_tick_distance(float(distance), _tier(tier), ctypes.byref(out))
    _check(status)
    return float(out.value)


def ladder_distance(distance: float, tiers: Sequence[Tier]) -> list[float]:
    tier_array = _tier_array(tiers)
    out = (ctypes.c_double * len(tiers))()
    status = _load_library().mc_ladder_distance(
        float(distance),
        tier_array,
        len(tiers),
        out,
        len(tiers),
    )
    _check(status)
    return [float(value) for value in out]


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


def smooth_ladder_distance(distance: float, tiers: Sequence[Tier], sharpness: float) -> list[float]:
    return [smooth_tick_distance(distance, tier, sharpness) for tier in tiers]


def adaptive_ladder_distance(
    distance: float,
    tiers: Sequence[Tier],
) -> tuple[list[float], ZoomDecision]:
    tier_array = _tier_array(tiers)
    out = (ctypes.c_double * len(tiers))()
    raw_decision = _MCZoomDecision()
    status = _load_library().mc_adaptive_ladder_distance(
        float(distance),
        tier_array,
        len(tiers),
        out,
        len(tiers),
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
    if not vectors:
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


class EventLog:
    def __init__(self, tier_count: int) -> None:
        self._ptr = _load_library().mc_event_log_new(tier_count)
        if not self._ptr:
            raise NativeStatusError(MC_STATUS_INVALID_ARGUMENT)

    @property
    def closed(self) -> bool:
        return self._ptr is None

    def close(self) -> None:
        if self._ptr is not None:
            _load_library().mc_event_log_free(self._ptr)
            self._ptr = None

    def append(self, state_id: int, ticks: Sequence[float]) -> int:
        self._ensure_open()
        if state_id < 0 or state_id > 2**64 - 1:
            raise ValueError("state_id must fit in uint64")
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

    def next_event(self, index: int, tier: int) -> Optional[int]:
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
