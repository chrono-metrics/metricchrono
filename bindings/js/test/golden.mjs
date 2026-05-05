import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { geometricLadder, ladderDistance, tickDistance, tier, weightedConsensus } from "../src/index.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../../crates/metricchrono-core");
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

assert.deepEqual(weightedConsensus([[1, 2], [3, 0]], [0.25, 0.75]), [2.5, 0.5]);

function readCsv(path) {
  return readFileSync(resolve(root, path), "utf8")
    .trim()
    .split("\n")
    .map((line) => line.split(","));
}

function number(value) {
  return Number.parseFloat(value);
}

function assertClose(name, actual, expected) {
  assert.ok(Math.abs(actual - expected) <= eps, `${name}: expected ${expected}, got ${actual}`);
}
