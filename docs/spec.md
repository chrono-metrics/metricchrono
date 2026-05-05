# MetricChrono Specification

MetricChrono is a multiscale metric-change ledger. It converts a nonnegative
metric distance into one or more deterministic tick values. It is not a clock,
causal model, planner, or claim that physical time is solved.

## Canonical Equation

For a valid tier with `epsilon > 0`, `delta > 0`, `epsilon < delta`,
finite `p`, and `epsilon_ref > 0`:

```text
T(d) = (epsilon / epsilon_ref)^p * ceil(d / delta) * 1[d >= epsilon]
```

`(epsilon / epsilon_ref)^p` is the gain. `ceil(d / delta)` is the active
staircase. `1[d >= epsilon]` is the activation gate.

## Parameters

- `d`: nonnegative finite metric distance.
- `epsilon`: activation threshold. Distances below this threshold are silent.
- `delta`: stair width. Active distances are mapped through `ceil(d / delta)`.
- `p`: gain exponent.
- `epsilon_ref`: positive reference scale for the gain.

## Hard Comparator Behavior

- If `d < epsilon`, the tick is `0`.
- If `d == epsilon`, the comparator is active.
- If `d > epsilon`, the comparator follows the active staircase.
- If `d == j * delta` and the comparator is active, the result is
  `gain * j`.

## Ladder Behavior

A ladder evaluates multiple valid tiers against the same distance and emits a
fixed-width tick vector. Tiers are ordered by increasing `epsilon` and `delta`.

## Smooth Surrogate

The smooth surrogate replaces the hard activation gate and staircase with
smooth approximations. It is for optimization and gradient-based experiments.
It is not identical to the hard comparator at activation or stair boundaries.
Gradients near those boundaries are surrogate gradients, not exact derivatives
of the hard map.

## Floating-Point Policy

Golden-vector tests use an absolute tolerance of `1e-12`. Checked APIs reject
negative, NaN, and infinite distances. Unchecked hot-path helpers assume valid
inputs and are intended for callers that already validated data.

## Golden Vectors

Single-scale manuscript vector:

| `d` | expected tick |
| ---: | ---: |
| `0.05` | `0` |
| `0.20` | `0.31622776601683794` |
| `0.60` | `0.6324555320336759` |
| `0.80` | `0.9486832980505138` |

Three-tier manuscript ladder:

| `d` | expected vector |
| ---: | --- |
| `0.05` | `[1, 0, 0]` |
| `0.20` | `[2, 1, 0]` |
| `1.00` | `[10, 4, 2]` |
| `3.00` | `[30, 10, 4]` |
