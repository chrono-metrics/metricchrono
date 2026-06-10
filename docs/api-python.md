# Python API

Python package:

```python
from metricchrono import Ladder, Tier, geometric_ladder, ladder_distance, tick_distance
```

The default Rust crate, C ABI, Python package, and JavaScript package expose the
same v0.2 keep-set: tick distance helpers, `Tier`/`TierBuilder`, ladder
construction and validation, ladder distance/value helpers, normalization and
carry rules, `EventLog`, smooth distances, adaptive ladder distance,
`weighted_consensus`, `Absolute`/`Euclidean` plus `MetricFn`, `tick_pair` and
`ladder_pair`, `PromotionCounter`, structured errors, and versioned schema
helpers.

Example:

```python
tier = Tier(epsilon=0.1, delta=0.3, p=0.5, epsilon_ref=1.0)
print(tick_distance(0.2, tier))

ladder = Ladder.geometric(0.03, 0.1, 3.0, 3, 0.0, 1.0)
print(ladder.values(1.0))
```

Expected output:

```text
0.31622776601683794
```

The Python package bundles the platform C ABI library in wheels. Source builds
require Cargo and a Rust toolchain.

`EventLog.append` follows the append-per-timestamp contract: call it once for
each observation, including quiet all-zero tick records. Tier events are the
positive-tick subset of those records. Use `first_event(tier)` to find the chain
head, then `next_event(index, tier)` or the compact summary/record readers to
walk the tier-local event chain.

Default metric helpers are limited to `Absolute`, `Euclidean`, `MetricFn`,
`tick_pair`, and `ladder_pair`. The six extra Rust metrics
`Cosine`, `KullbackLeibler`, `JensenShannon`, `Manhattan`,
`SquaredEuclidean`, and `DiagonalMahalanobis` require the Rust
`metrics-extra` feature and are absent from the default bindings.

Versioned schema helpers are exported as `tier_from_schema`,
`ladder_from_schema`, `tick_vector_from_schema`, and
`consensus_result_from_schema`, with matching `*_to_schema` helpers where the
Python API owns the value shape.

## Coverage meter

```python
import metricchrono as mc

meter = mc.CoverageMeter([0.1, 0.2], dim=2)            # built-in euclidean
flags = meter.observe([0.0, 0.0])                      # per-tier admission
meter.counts, meter.unique_representatives

chebyshev = lambda a, b: max(abs(x - y) for x, y in zip(a, b))
custom = mc.CoverageMeter([0.1], dim=2, metric=chebyshev)  # any callable

mc.classify_regime(throughput_delta, coverage_delta)   # OperatingRegime
mc.progress_efficiency(coverage, epsilon, path_length)
```

Coverage is the revisit-invariant complement to tick throughput; jointly they
classify windows as quiescent / progress / churn / creep. Exceptions raised by
a callable metric are converted to NaN, which rejects admission instead of
unwinding into the C ABI.
