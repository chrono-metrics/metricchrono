from __future__ import annotations

import ctypes
import os
import sys
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Sequence

MC_STATUS_OK = 0
MC_STATUS_NULL = 1
MC_STATUS_INVALID_ARGUMENT = 2
MC_STATUS_BUFFER_TOO_SMALL = 3
MC_STATUS_PANIC = 255


class NativeLoadError(RuntimeError):
    """Raised when the MetricChrono shared library cannot be loaded."""


class NativeStatusError(RuntimeError):
    """Raised when the MetricChrono C ABI returns an error status."""


@dataclass(frozen=True)
class Tier:
    epsilon: float
    delta: float
    p: float = 0.5
    epsilon_ref: float = 1.0


class _MCTier(ctypes.Structure):
    _fields_ = [
        ("epsilon", ctypes.c_double),
        ("delta", ctypes.c_double),
        ("p", ctypes.c_double),
        ("epsilon_ref", ctypes.c_double),
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

    lib.mc_weighted_consensus.argtypes = [
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_double),
        ctypes.POINTER(ctypes.c_double),
        ctypes.c_size_t,
    ]
    lib.mc_weighted_consensus.restype = ctypes.c_int


def _tier(value: Tier) -> _MCTier:
    return _MCTier(value.epsilon, value.delta, value.p, value.epsilon_ref)


def _check(status: int) -> None:
    if status == MC_STATUS_OK:
        return
    names = {
        MC_STATUS_NULL: "null pointer",
        MC_STATUS_INVALID_ARGUMENT: "invalid argument",
        MC_STATUS_BUFFER_TOO_SMALL: "buffer too small",
        MC_STATUS_PANIC: "panic",
    }
    raise NativeStatusError(names.get(status, f"unknown status {status}"))


def tick_distance(distance: float, tier: Tier) -> float:
    out = ctypes.c_double()
    status = _load_library().mc_tick_distance(float(distance), _tier(tier), ctypes.byref(out))
    _check(status)
    return float(out.value)


def ladder_distance(distance: float, tiers: Sequence[Tier]) -> list[float]:
    tier_array = (_MCTier * len(tiers))(*[_tier(tier) for tier in tiers])
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


def weighted_consensus(vectors: Sequence[Sequence[float]], weights: Sequence[float]) -> list[float]:
    if not vectors:
        raise ValueError("vectors must not be empty")
    cols = len(vectors[0])
    if any(len(row) != cols for row in vectors):
        raise ValueError("all vectors must have the same length")
    flat = [float(value) for row in vectors for value in row]
    flat_array = (ctypes.c_double * len(flat))(*flat)
    weight_array = (ctypes.c_double * len(weights))(*[float(value) for value in weights])
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
