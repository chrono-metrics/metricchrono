# MetricChrono JS

This package gives browser and Node users the same public kernel shape as the
Rust crate.

It includes:

- A zero-dependency JS reference implementation for golden tests and lightweight
  browser use.
- `loadWasmMetricChrono`, a tiny loader for a future `wasm32` build that exports
  `mc_tick_distance_raw`.

Run the golden test:

```sh
npm test
```
