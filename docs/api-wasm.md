# JavaScript API (Optional WASM Interop)

JavaScript package:

```ts
import { tier, geometricLadder, tickDistance, ladderDistance } from "@metricchrono/core";
```

The public JavaScript package is dependency-free and includes TypeScript
definitions. The default Rust crate, C ABI, Python package, and JavaScript
package expose the same current default surface: tick distance helpers,
`Tier`/`tier`, ladder construction and validation, ladder distance/value
helpers, normalization and carry rules, `EventLog`, smooth distances, adaptive
ladder distance, `CoverageMeter`, `progressEfficiency`, `classifyRegime`,
`weightedConsensus`, `Absolute`/`Euclidean` plus custom metric callbacks,
`tickPair` and `ladderPair`, `PromotionCounter`, structured errors, and
versioned schema helpers.

`EventLog.append` follows the append-per-timestamp contract: call it once for
each observation, including quiet all-zero tick records. Tier events are the
positive-tick subset of those records. Use `firstEvent(tier)` to find the chain
head, then `nextEvent(index, tier)` or the compact summary/record readers to
walk the tier-local event chain.

Versioned schema helpers are exported as `tierFromSchema`, `ladderFromSchema`,
`tickVectorFromSchema`, and `consensusResultFromSchema`, with matching
`*ToSchema` helpers for local values.

Default metric helpers are limited to `absoluteDistance`,
`euclideanDistance`, custom metric callbacks, `tickPair`, and `ladderPair`. The
six extra Rust metrics `Cosine`, `KullbackLeibler`, `JensenShannon`,
`Manhattan`, `SquaredEuclidean`, and `DiagonalMahalanobis` require the Rust
`metrics-extra` feature and are absent from the default bindings.

`loadWasmMetricChrono` can wrap a compatible WASM module that exports
`mc_tick_distance_raw`. No separate optimized WASM runtime is claimed; the
published package is pure JS with an optional WASM-interop hook
(`loadWasmMetricChrono`).
