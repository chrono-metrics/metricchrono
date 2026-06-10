import assert from "node:assert/strict";

import {
  CoverageMeter,
  MetricChronoError,
  OperatingRegime,
  absoluteDistance,
  classifyRegime,
  progressEfficiency,
} from "../src/index.js";

// round trip with pooled storage
{
  const meter = new CoverageMeter([0.1, 0.2]);
  assert.deepEqual(meter.observe([0.0, 0.0]), [true, true]);
  // 0.15 away: tier 0 admits (>= 0.1), tier 1 rejects (< 0.2)
  assert.deepEqual(meter.observe([0.15, 0.0]), [true, false]);
  assert.deepEqual(meter.counts(), [2, 1]);
  assert.equal(meter.uniqueRepresentatives(), 2);
  assert.deepEqual(meter.representatives(1), [[0.0, 0.0]]);
}

// creep: sub-threshold relocation registers in coverage
{
  const meter = new CoverageMeter([0.1], absoluteDistance);
  let position = 0.0;
  let admittedTotal = 0;
  meter.observe(position);
  for (let step = 0; step < 100; step += 1) {
    position += 0.05;
    admittedTotal += meter.observe(position).filter(Boolean).length;
  }
  assert.ok(meter.count(0) > 30, `coverage must register creep, got ${meter.count(0)}`);
  assert.equal(classifyRegime(0.0, admittedTotal), OperatingRegime.Creep);
}

// churn: supra-threshold bouncing freezes coverage at 2
{
  const meter = new CoverageMeter([0.1], absoluteDistance);
  for (let step = 0; step < 50; step += 1) {
    meter.observe(step % 2 === 0 ? 0.0 : 0.5);
  }
  assert.equal(meter.count(0), 2);
  assert.equal(classifyRegime(42.0, 0), OperatingRegime.Churn);
}

// quadrants and efficiency
{
  assert.equal(classifyRegime(0.0, 0), OperatingRegime.Quiescent);
  assert.equal(classifyRegime(1.0, 1), OperatingRegime.Progress);
  assert.ok(Math.abs(progressEfficiency(11, 0.1, 2.0) - 0.5) < 1e-12);
  assert.throws(() => progressEfficiency(11, 0.1, 0.0), MetricChronoError);
}

// invalid construction and NaN rejection
{
  assert.throws(() => new CoverageMeter([]), MetricChronoError);
  assert.throws(() => new CoverageMeter([0.0]), MetricChronoError);
  assert.throws(() => new CoverageMeter([0.1], "not-a-function"), MetricChronoError);
  const meter = new CoverageMeter([0.1], () => Number.NaN);
  meter.observe([0.0]);
  meter.observe([10.0]);
  assert.equal(meter.count(0), 1, "NaN distances must reject admission");
}

console.log("coverage.mjs: all assertions passed");
