import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import {
  EventLog,
  Normalization,
  PromotionCounter,
  adaptiveLadderDistance,
  adaptiveZoomWindow,
  carryRules,
  coherenceResidual,
  coherenceResiduals,
  consensusResultFromSchema,
  cosineDistance,
  diagonalMahalanobisDistance,
  euclideanDistance,
  geometricLadder,
  jensenShannonDistance,
  kullbackLeiblerDistance,
  ladderFromSchema,
  ladderDistance,
  ladderToSchema,
  ladderPair,
  manhattanDistance,
  normalizeTicks,
  simpleWeightUpdate,
  smoothLadderDistance,
  smoothTickDistance,
  squaredEuclideanDistance,
  tickDistance,
  tickVectorFromSchema,
  tickVectorToSchema,
  tickPair,
  tier,
  tierFromSchema,
  tierToSchema,
  weightedConsensus,
} from "../src/index.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../../crates/metricchrono-core");
const repoRoot = resolve(root, "../..");
const eps = 1e-12;

for (const row of readCsv("fixtures/golden_ticks.csv").slice(1)) {
  const t = tier(number(row[2]), number(row[3]), number(row[4]), number(row[5]));
  assertClose(row[0], tickDistance(number(row[1]), t), number(row[6]));
}

for (const row of readCsv("fixtures/golden_ladders.csv").slice(1)) {
  const ladder = geometricLadder(
    number(row[2]),
    number(row[3]),
    number(row[4]),
    Number.parseInt(row[5], 10),
    number(row[6]),
    number(row[7]),
  );
  const actual = ladderDistance(number(row[1]), ladder);
  const expected = row[8].split(";").map(number);
  assert.equal(actual.length, expected.length, row[0]);
  for (let index = 0; index < actual.length; index += 1) {
    assertClose(`${row[0]}[${index}]`, actual[index], expected[index]);
  }
}

const ladder = geometricLadder(0.5, 1.0, 2.0, 4, 0.5, 1.0);
assert.equal(smoothTickDistance(0.95, tier(1.0, 2.0, 0.5, 1.0), 10.0) > 0, true);
assert.equal(smoothLadderDistance(2.0, ladder, 10.0).length, 4);
assert.deepEqual(normalizeTicks([10, 5, 3], Normalization.UnitMax), [1, 0.5, 0.3]);
assert.deepEqual(carryRules([0.1, 1.2, 3.0]), [1, 2, 3]);

const promote = new PromotionCounter([2, 3]);
assert.deepEqual(promote.step([false, false]), [false, false]);
assert.deepEqual(promote.counters, [1, 1]);
assert.deepEqual(promote.step([false, false]), [true, true]);
assert.deepEqual(promote.counters, [0, 0]);

const adaptive = adaptiveLadderDistance(0.75, ladder);
assert.equal(adaptive.decision.firstInactiveTier, 1);
assert.equal(adaptive.decision.stoppedEarly, true);
assert.deepEqual(adaptive.ticks.slice(1), [0, 0, 0]);
assert.deepEqual(adaptiveZoomWindow(3.0, ladder, 1), [1, 4]);

const log = new EventLog(3);
assert.equal(log.append("s0", [0, 0, 0]), 0);
assert.equal(log.append("s1", [1, 0, 0]), 1);
assert.equal(log.append("s2", [0, 2, 0]), 2);
assert.equal(log.append("s3", [1, 1, 0]), 3);
assert.equal(log.firstEvent(0), 1);
assert.equal(log.nextEvent(1, 0), 3);
assert.deepEqual(
  Array.from(log.iterEvents(0), ([index]) => index),
  [1, 3],
);
assert.deepEqual(log.compactSummary(1).map((item) => item.stateId), ["s2", "s3"]);

const consensus = weightedConsensus(
  [
    [1, 2],
    [3, 0],
  ],
  [0.25, 0.75],
);
assert.deepEqual(consensus, [2.5, 0.5]);
assert.equal(coherenceResidual([1, 2], consensus) > 0, true);
const residuals = coherenceResiduals(
  [
    [1, 2],
    [3, 0],
  ],
  consensus,
);
assertClose("updated weights sum", simpleWeightUpdate([0.5, 0.5], residuals, 0.2, 0.01).reduce(sum, 0), 1);

assertClose("euclidean", euclideanDistance([0, 0], [3, 4]), 5);
assertClose("squaredEuclidean", squaredEuclideanDistance([0, 0], [3, 4]), 25);
assertClose("manhattan", manhattanDistance([0, 0], [3, 4]), 7);
assertClose("cosine", cosineDistance([1, 0], [0, 1]), 1);
assert.equal(kullbackLeiblerDistance([0.2, 0.8], [0.5, 0.5]) > 0, true);
assert.equal(jensenShannonDistance([0.2, 0.8], [0.5, 0.5]) > 0, true);
assertClose(
  "diagonalMahalanobis",
  diagonalMahalanobisDistance([0, 0], [4, 3], [0.25, 1.0]),
  Math.sqrt(13),
);
assertClose("tickPair", tickPair([0, 0], [3, 4], euclideanDistance, tier(0.5, 1.0, 0, 1)), 5);
assert.throws(() => tier(1.0, 1.0, 0, 1));
assert.throws(() => tickDistance(-1, tier(0.5, 1.0, 0, 1)));
assert.deepEqual(ladderPair([0, 0], [3, 4], euclideanDistance, ladder).length, 4);

const tierDoc = {
  metricchrono_schema: "tier.v1",
  epsilon: 0.03,
  delta: 0.1,
  p: 0.0,
  epsilon_ref: 1.0,
};
const schemaTier = tierFromSchema(tierDoc);
assert.deepEqual(tierToSchema(schemaTier), tierDoc);

const ladderDoc = readJson("tests/golden/ladder.v1.json");
const schemaLadder = ladderFromSchema(ladderDoc);
assert.deepEqual(ladderDistance(1.0, schemaLadder), [10, 4, 2]);
assert.deepEqual(ladderToSchema(schemaLadder), ladderDoc);

const tickDoc = readJson("tests/golden/tick_vector.v1.json");
const schemaTicks = tickVectorFromSchema(tickDoc);
assert.deepEqual(schemaTicks, [10, 4, 2]);
assert.deepEqual(tickVectorToSchema(schemaTicks), tickDoc);

const consensusDoc = readJson("tests/golden/consensus_result.v1.json");
assert.deepEqual(consensusResultFromSchema(consensusDoc), consensusDoc);

assert.throws(() => ladderFromSchema({ ...ladderDoc, metricchrono_schema: "ladder.v2" }));

function readCsv(path) {
  return readFileSync(resolve(root, path), "utf8")
    .trim()
    .split("\n")
    .map((line) => line.split(","));
}

function readJson(path) {
  return JSON.parse(readFileSync(resolve(repoRoot, path), "utf8"));
}

function number(value) {
  return Number.parseFloat(value);
}

function assertClose(name, actual, expected) {
  assert.ok(Math.abs(actual - expected) <= eps, `${name}: expected ${expected}, got ${actual}`);
}

function sum(left, right) {
  return left + right;
}
