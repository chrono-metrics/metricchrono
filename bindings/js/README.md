# MetricChrono JS

Production JavaScript bindings for the MetricChrono public kernel. The package
is dependency-free, typed, and works in Node and modern browsers.

## Install From Source

```sh
npm install ./bindings/js
```

## Example

```js
import {
  EventLog,
  geometricLadder,
  ladderDistance,
  tier,
  tickDistance,
  weightedConsensus,
} from "@metricchrono/core";

const t = tier(0.5, 1.0, 0.5, 1.0);
console.log(tickDistance(1.2, t));

const ladder = geometricLadder(0.5, 1.0, 2.0, 4, 0.5, 1.0);
console.log(ladderDistance(3.0, ladder));

const log = new EventLog(2);
log.append("s0", [0, 0]);
log.append("s1", [1, 0]);

console.log(weightedConsensus([[1, 2], [3, 0]], [0.25, 0.75]));
```

## API Surface

- Kernel and ladders: `tier`, `tickDistance`, `ladderDistance`,
  `geometricLadder`, `customLadder`, `validateLadder`.
- Serialization: `tierFromSchema`, `tierToSchema`, `ladderFromSchema`,
  `ladderToSchema`, `tickVectorFromSchema`, `tickVectorToSchema`,
  `consensusResultFromSchema`.
- Smooth and adaptive helpers: `smoothTickDistance`, `smoothLadderDistance`,
  `adaptiveLadderDistance`, `adaptiveZoomWindow`.
- Event memory: `EventLog`.
- Carry and normalization: `PromotionCounter`, `carryRules`,
  `normalizeTicks`.
- Metrics: scalar absolute and Euclidean distances, custom metric callbacks,
  `tickPair`, and `ladderPair`.
- Consensus: `weightedConsensus`, `coherenceResiduals`, and
  `simpleWeightUpdate`.
- Optional interop: `loadWasmMetricChrono` can wrap a compatible WASM module
  that exports `mc_tick_distance_raw`.

## Verify

```sh
npm test
npm pack --dry-run
```
