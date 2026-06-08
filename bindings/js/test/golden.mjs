import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import {
  EventLog,
  Normalization,
  PromotionCounter,
  absoluteDistance,
  adaptiveLadderDistance,
  carryRules,
  consensusResultFromSchema,
  customLadder,
  euclideanDistance,
  geometricLadder,
  ladderDistance,
  ladderFromSchema,
  ladderPair,
  ladderToSchema,
  normalizeTicks,
  smoothLadderDistance,
  smoothTickDistance,
  tickDistance,
  tickPair,
  tickVectorFromSchema,
  tickVectorToSchema,
  tier,
  tierFromSchema,
  tierToSchema,
  validateLadder,
  weightedConsensus,
} from "../src/index.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../../crates/metricchrono-core");
const repoRoot = resolve(root, "../..");
const eps = 1e-12;
const fixture = readJson(resolve(root, "fixtures/binding_conformance.v1.json"));

assert.equal(fixture.metricchrono_schema, "binding_conformance.v1");

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
  assertVectorClose(row[0], actual, expected);
}

const ladders = buildLadders(fixture);

for (const testCase of fixture.tick_distance_cases) {
  const t = tierFromDoc(testCase.tier);
  assertClose(testCase.name, tickDistance(testCase.distance, t), testCase.expected);
}

for (const testCase of fixture.smooth_tick_distance_cases) {
  assertClose(
    testCase.name,
    smoothTickDistance(testCase.distance, tierFromDoc(testCase.tier), testCase.sharpness),
    testCase.expected,
  );
}

for (const testCase of fixture.smooth_ladder_distance_cases) {
  assertVectorClose(
    testCase.name,
    smoothLadderDistance(testCase.distance, ladders.get(testCase.ladder), testCase.sharpness),
    testCase.expected,
  );
}

for (const testCase of fixture.adaptive_ladder_distance_cases) {
  const actual = adaptiveLadderDistance(testCase.distance, ladders.get(testCase.ladder));
  assertVectorClose(testCase.name, actual.ticks, testCase.expected.ticks);
  assert.equal(actual.decision.evaluatedTiers, testCase.expected.decision.evaluated_tiers);
  assert.equal(actual.decision.firstInactiveTier, testCase.expected.decision.first_inactive_tier);
  assert.equal(actual.decision.stoppedEarly, testCase.expected.decision.stopped_early);
}

for (const testCase of fixture.weighted_consensus_cases) {
  assertVectorClose(
    testCase.name,
    weightedConsensus(testCase.vectors, testCase.weights),
    testCase.expected,
  );
}

for (const testCase of fixture.event_log_cases) {
  assertEventLogCase(testCase);
}

for (const testCase of fixture.metric_cases.euclidean_distance) {
  assertClose(testCase.name, euclideanDistance(testCase.a, testCase.b), testCase.expected);
}

for (const testCase of fixture.metric_cases.absolute_distance) {
  assertClose(testCase.name, absoluteDistance(testCase.a, testCase.b), testCase.expected);
}

for (const testCase of fixture.pair_cases.tick_pair) {
  assertClose(
    testCase.name,
    tickPair(testCase.a, testCase.b, metricFromName(testCase.metric), tierFromDoc(testCase.tier)),
    testCase.expected,
  );
}

for (const testCase of fixture.pair_cases.ladder_pair) {
  assertVectorClose(
    testCase.name,
    ladderPair(testCase.a, testCase.b, metricFromName(testCase.metric), ladders.get(testCase.ladder)),
    testCase.expected,
  );
}

for (const testCase of fixture.normalize_ticks_cases) {
  assertVectorClose(
    testCase.name,
    normalizeTicks(testCase.input, normalizationMode(testCase.mode)),
    testCase.expected,
  );
}

for (const testCase of fixture.carry_rules_cases) {
  assert.deepEqual(carryRules(testCase.epsilons), testCase.expected, testCase.name);
}

for (const testCase of fixture.promotion_counter_cases) {
  const counter = new PromotionCounter(testCase.quotas);
  assert.deepEqual(counter.quotas, testCase.quotas, `${testCase.name}/quotas`);
  for (const step of testCase.steps) {
    const actual = step.event_flags === null ? counter.step() : counter.step(step.event_flags);
    assert.deepEqual(actual, step.promoted, `${testCase.name}/promoted`);
    assert.deepEqual(counter.counters, step.counters, `${testCase.name}/counters`);
  }
  counter.reset();
  assert.deepEqual(counter.counters, testCase.after_reset_counters, `${testCase.name}/reset`);
}

assertRejections(fixture.rejections, fixture.event_log_cases[0]);
assertSchemaRoundTrip();

function buildLadders(source) {
  const out = new Map();
  for (const testCase of source.ladders) {
    const expected = testCase.tiers.map(tierFromDoc);
    let actual;
    if (testCase.kind === "geometric") {
      const params = testCase.params;
      actual = geometricLadder(
        params.epsilon0,
        params.delta0,
        params.ratio,
        params.tiers,
        params.p,
        params.epsilon_ref,
      );
    } else if (testCase.kind === "custom") {
      actual = customLadder(expected);
    } else {
      throw new Error(`unknown ladder kind ${testCase.kind}`);
    }

    validateLadder(actual);
    assert.equal(actual.length, expected.length, `${testCase.name}/tiers`);
    for (let index = 0; index < actual.length; index += 1) {
      assertClose(`${testCase.name}[${index}].epsilon`, actual[index].epsilon, expected[index].epsilon);
      assertClose(`${testCase.name}[${index}].delta`, actual[index].delta, expected[index].delta);
      assertClose(`${testCase.name}[${index}].p`, actual[index].p, expected[index].p);
      assertClose(
        `${testCase.name}[${index}].epsilonRef`,
        actual[index].epsilonRef,
        expected[index].epsilonRef,
      );
    }
    for (const distanceCase of testCase.distances) {
      assertVectorClose(
        `${testCase.name}/ladderDistance`,
        ladderDistance(distanceCase.distance, actual),
        distanceCase.expected,
      );
    }
    out.set(testCase.name, actual);
  }
  return out;
}

function assertEventLogCase(testCase) {
  const log = new EventLog(testCase.tier_count);
  assert.equal(log.length === 0, testCase.is_empty_before_append, `${testCase.name}/isEmptyBefore`);
  for (const record of testCase.append_records) {
    assert.equal(log.append(record.state_id, record.ticks), record.expected_index);
  }
  assert.equal(log.length, testCase.expected_len, `${testCase.name}/length`);
  assert.equal(log.length === 0, testCase.is_empty_after_append, `${testCase.name}/isEmptyAfter`);
  assert.equal(log.tierCount, testCase.tier_count, `${testCase.name}/tierCount`);

  for (const expected of testCase.records) {
    const actual = log.records[expected.index];
    assert.equal(actual.stateId, expected.state_id, `${testCase.name}/recordState`);
    assertVectorClose(`${testCase.name}/record[${expected.index}]`, actual.ticks, expected.ticks);
  }

  for (const expected of testCase.first_events) {
    assert.equal(log.firstEvent(expected.tier), expected.expected, `${testCase.name}/firstEvent`);
  }

  for (const expected of testCase.next_events) {
    assert.equal(
      log.nextEvent(expected.index, expected.tier),
      expected.expected,
      `${testCase.name}/nextEvent`,
    );
  }

  for (const summary of testCase.compact_summaries) {
    const actual = log.compactSummary(summary.tier);
    assert.equal(actual.length, summary.expected.length, `${testCase.name}/summaryLength`);
    for (let index = 0; index < actual.length; index += 1) {
      assert.equal(actual[index].index, summary.expected[index].index);
      assert.equal(actual[index].stateId, summary.expected[index].state_id);
      assertClose("compact summary tick", actual[index].tick, summary.expected[index].tick);
    }
  }
}

function assertRejections(rejections, eventCase) {
  for (const testCase of rejections.invalid_tiers) {
    assert.throws(
      () => tier(testCase.epsilon, testCase.delta, testCase.p, testCase.epsilon_ref),
      undefined,
      testCase.name,
    );
  }

  for (const testCase of rejections.unknown_schema_documents) {
    if (testCase.kind === "tier") {
      assert.throws(() => tierFromSchema(testCase.document), undefined, testCase.name);
    } else {
      throw new Error(`unknown schema rejection kind ${testCase.kind}`);
    }
  }

  const log = new EventLog(eventCase.tier_count);
  for (const record of eventCase.append_records) {
    log.append(record.state_id, record.ticks);
  }
  for (const testCase of rejections.event_log_out_of_range) {
    if (testCase.operation === "record") {
      assert.throws(() => {
        const record = log.records[testCase.index];
        if (record === undefined) {
          throw new Error("event log index is out of bounds");
        }
      }, undefined, testCase.name);
    } else if (testCase.operation === "next_event") {
      assert.throws(() => log.nextEvent(testCase.index, testCase.tier), undefined, testCase.name);
    } else if (testCase.operation === "first_event") {
      assert.throws(() => log.firstEvent(testCase.tier), undefined, testCase.name);
    } else if (testCase.operation === "compact_summary") {
      assert.throws(() => log.compactSummary(testCase.tier), undefined, testCase.name);
    } else {
      throw new Error(`unknown event log rejection operation ${testCase.operation}`);
    }
  }
}

function assertSchemaRoundTrip() {
  const tierDoc = {
    metricchrono_schema: "tier.v1",
    epsilon: 0.03,
    delta: 0.1,
    p: 0.0,
    epsilon_ref: 1.0,
  };
  const schemaTier = tierFromSchema(tierDoc);
  assert.deepEqual(tierToSchema(schemaTier), tierDoc);

  const ladderDoc = readJson(resolve(repoRoot, "tests/golden/ladder.v1.json"));
  const schemaLadder = ladderFromSchema(ladderDoc);
  assert.deepEqual(ladderDistance(1.0, schemaLadder), [10, 4, 2]);
  assert.deepEqual(ladderToSchema(schemaLadder), ladderDoc);

  const tickDoc = readJson(resolve(repoRoot, "tests/golden/tick_vector.v1.json"));
  const schemaTicks = tickVectorFromSchema(tickDoc);
  assert.deepEqual(schemaTicks, [10, 4, 2]);
  assert.deepEqual(tickVectorToSchema(schemaTicks), tickDoc);

  const consensusDoc = readJson(resolve(repoRoot, "tests/golden/consensus_result.v1.json"));
  assert.deepEqual(consensusResultFromSchema(consensusDoc), consensusDoc);

  assert.throws(() => ladderFromSchema({ ...ladderDoc, metricchrono_schema: "ladder.v2" }));
}

function metricFromName(name) {
  if (name === "euclidean") {
    return euclideanDistance;
  }
  if (name === "absolute") {
    return absoluteDistance;
  }
  if (name === "max_abs") {
    return maxAbsDistance;
  }
  throw new Error(`unknown metric ${name}`);
}

function maxAbsDistance(a, b) {
  if (a.length !== b.length) {
    return Number.NaN;
  }
  return a.map((value, index) => Math.abs(value - b[index])).reduce((left, right) => Math.max(left, right), 0);
}

function normalizationMode(mode) {
  if (mode === "none") {
    return Normalization.None;
  }
  if (mode === "unit_max") {
    return Normalization.UnitMax;
  }
  if (mode === "tanh") {
    return Normalization.Tanh;
  }
  throw new Error(`unknown normalization ${mode}`);
}

function tierFromDoc(document) {
  return tier(document.epsilon, document.delta, document.p, document.epsilon_ref);
}

function readCsv(path) {
  return readFileSync(resolve(root, path), "utf8")
    .trim()
    .split("\n")
    .map((line) => line.split(","));
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function number(value) {
  return Number.parseFloat(value);
}

function assertVectorClose(name, actual, expected) {
  assert.equal(actual.length, expected.length, `${name}: length mismatch`);
  for (let index = 0; index < actual.length; index += 1) {
    assertClose(`${name}[${index}]`, actual[index], expected[index]);
  }
}

function assertClose(name, actual, expected) {
  assert.ok(Math.abs(actual - expected) <= eps, `${name}: expected ${expected}, got ${actual}`);
}
