# Python API

Python package:

```python
from metricchrono import Tier, geometric_ladder, ladder_distance, tick_distance
```

Example:

```python
tier = Tier(epsilon=0.1, delta=0.3, p=0.5, epsilon_ref=1.0)
print(tick_distance(0.2, tier))
```

Expected output:

```text
0.31622776601683794
```

The Python package bundles the platform C ABI library in wheels. Source builds
require Cargo and a Rust toolchain.

Versioned schema helpers are exported as `tier_from_schema`,
`ladder_from_schema`, `tick_vector_from_schema`, and
`consensus_result_from_schema`, with matching `*_to_schema` helpers where the
Python API owns the value shape.
