# Edge Cases

## Activation Edge

The activation gate is closed for `d < epsilon` and open for `d >= epsilon`.
This means `d == epsilon` is active.

## Stair Edge

For active inputs, exact multiples of `delta` map to the exact stair index:

```text
d == j * delta => tick == gain * j
```

## Invalid Parameters

Checked constructors reject:

- nonfinite `epsilon`, `delta`, `p`, or `epsilon_ref`;
- `epsilon <= 0`;
- `delta <= 0`;
- `epsilon >= delta`;
- `epsilon_ref <= 0`.

## Invalid Distances

Checked APIs reject negative, NaN, and infinite distances. Unchecked helpers are
reserved for already validated hot paths.

## Non-Additivity

MetricChrono encodes net metric change at a chosen scale. In general:

```text
T(d1 + d2) != T(d1) + T(d2)
```

Do not use tick values as elapsed-time accumulators.
