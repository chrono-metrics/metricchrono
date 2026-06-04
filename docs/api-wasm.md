# WASM / JavaScript API

JavaScript package:

```ts
import { tier, geometricLadder, tickDistance, ladderDistance } from "@metricchrono/core";
```

The public JavaScript package is dependency-free and includes TypeScript
definitions. It mirrors the public kernel, ladder, smooth surrogate, adaptive
zoom, event log, basic metric, and minimal consensus APIs.

Versioned schema helpers are exported as `tierFromSchema`, `ladderFromSchema`,
`tickVectorFromSchema`, and `consensusResultFromSchema`, with matching
`*ToSchema` helpers for local values.

Metric helpers include scalar `absoluteDistance`, vector `euclideanDistance`,
and the full browser/JS metric set: `squaredEuclideanDistance`,
`manhattanDistance`, `cosineDistance`, `kullbackLeiblerDistance`,
`jensenShannonDistance`, and `diagonalMahalanobisDistance`. The
`metrics-extra` flag is only a Rust `metricchrono-core` Cargo feature; the
browser/JS-WASM wrapper exposes its metric helpers directly without requiring a
Cargo feature.

`loadWasmMetricChrono` can wrap a compatible WASM module that exports
`mc_tick_distance_raw`. The repository does not claim a separate optimized WASM
runtime for v0.2.0; the browser-ready JavaScript implementation is the public
wrapper.
