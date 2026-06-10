export class MetricChronoError extends Error {
  constructor(message) {
    super(message);
    this.name = "MetricChronoError";
  }
}

export const Normalization = Object.freeze({
  None: "none",
  UnitMax: "unitMax",
  Tanh: "tanh",
});

export function tier(epsilon, delta, p = 0.5, epsilonRef = 1.0) {
  const value = { epsilon, delta, p, epsilonRef };
  validateTier(value);
  return Object.freeze(value);
}

export function validateTier(tierSpec, index = 0) {
  if (!Number.isFinite(tierSpec.epsilon) || tierSpec.epsilon <= 0) {
    throw new MetricChronoError(`invalid tier at index ${index}: epsilon must be finite and > 0`);
  }
  if (!Number.isFinite(tierSpec.delta) || tierSpec.delta <= 0) {
    throw new MetricChronoError(`invalid tier at index ${index}: delta must be finite and > 0`);
  }
  if (tierSpec.epsilon >= tierSpec.delta) {
    throw new MetricChronoError(`invalid tier at index ${index}: epsilon must be < delta`);
  }
  if (!Number.isFinite(tierSpec.p)) {
    throw new MetricChronoError(`invalid tier at index ${index}: p must be finite`);
  }
  if (!Number.isFinite(tierSpec.epsilonRef) || tierSpec.epsilonRef <= 0) {
    throw new MetricChronoError(
      `invalid tier at index ${index}: epsilonRef must be finite and > 0`,
    );
  }
}

export function validateLadder(ladder) {
  if (ladder.length === 0) {
    throw new MetricChronoError("ladder must contain at least one tier");
  }
  for (let index = 0; index < ladder.length; index += 1) {
    validateTier(ladder[index], index);
    if (index > 0 && ladder[index].epsilon <= ladder[index - 1].epsilon) {
      throw new MetricChronoError(
        `invalid tier at index ${index}: epsilon values must be strictly increasing`,
      );
    }
    if (index > 0 && ladder[index].delta <= ladder[index - 1].delta) {
      throw new MetricChronoError(
        `invalid tier at index ${index}: delta values must be strictly increasing`,
      );
    }
  }
}

export function tickDistance(distance, tierSpec) {
  validateTier(tierSpec);
  assertDistance(distance);
  const d = distance;
  if (d < tierSpec.epsilon) {
    return 0;
  }
  const gain = Math.pow(tierSpec.epsilon / tierSpec.epsilonRef, tierSpec.p);
  return finiteOrMax(gain * Math.ceil(d / tierSpec.delta));
}

export function ladderDistance(distance, ladder) {
  validateLadder(ladder);
  return ladder.map((tierSpec) => tickDistance(distance, tierSpec));
}

export function customLadder(tiers) {
  validateLadder(tiers);
  return tiers.map((tierSpec) =>
    tier(tierSpec.epsilon, tierSpec.delta, tierSpec.p, tierSpec.epsilonRef),
  );
}

export function geometricLadder(epsilon0, delta0, ratio, tiers, p = 0.5, epsilonRef = 1.0) {
  if (!Number.isInteger(tiers) || tiers <= 0) {
    throw new MetricChronoError("tiers must be a positive integer");
  }
  if (!Number.isFinite(ratio) || ratio <= 1) {
    throw new MetricChronoError("ratio must be finite and > 1");
  }
  return customLadder(
    Array.from({ length: tiers }, (_, index) => {
      const scale = Math.pow(ratio, index);
      return { epsilon: epsilon0 * scale, delta: delta0 * scale, p, epsilonRef };
    }),
  );
}

export function tierFromSchema(document) {
  ensureSchema(document, "tier.v1", ["metricchrono_schema", "epsilon", "delta", "p", "epsilon_ref"]);
  return tier(document.epsilon, document.delta, document.p, document.epsilon_ref);
}

export function tierToSchema(tierSpec) {
  validateTier(tierSpec);
  return {
    metricchrono_schema: "tier.v1",
    epsilon: tierSpec.epsilon,
    delta: tierSpec.delta,
    p: tierSpec.p,
    epsilon_ref: tierSpec.epsilonRef,
  };
}

export function ladderFromSchema(document) {
  ensureSchema(document, "ladder.v1", ["metricchrono_schema", "tiers"]);
  return customLadder(
    document.tiers.map((item, index) => {
      ensureExactFields(item, ["epsilon", "delta", "p", "epsilon_ref"], `tier at index ${index}`);
      return {
        epsilon: item.epsilon,
        delta: item.delta,
        p: item.p,
        epsilonRef: item.epsilon_ref,
      };
    }),
  );
}

export function ladderToSchema(ladder) {
  validateLadder(ladder);
  return {
    metricchrono_schema: "ladder.v1",
    tiers: ladder.map((tierSpec) => ({
      epsilon: tierSpec.epsilon,
      delta: tierSpec.delta,
      p: tierSpec.p,
      epsilon_ref: tierSpec.epsilonRef,
    })),
  };
}

export function tickVectorFromSchema(document) {
  ensureSchema(document, "tick_vector.v1", ["metricchrono_schema", "ticks"]);
  return document.ticks.slice();
}

export function tickVectorToSchema(ticks) {
  return {
    metricchrono_schema: "tick_vector.v1",
    ticks: ticks.slice(),
  };
}

export function consensusResultFromSchema(document) {
  ensureSchema(document, "consensus_result.v1", [
    "metricchrono_schema",
    "consensus",
    "residuals",
    "weights",
  ]);
  return {
    metricchrono_schema: "consensus_result.v1",
    consensus: document.consensus.slice(),
    residuals: document.residuals.slice(),
    weights: document.weights.slice(),
  };
}

export function smoothTickDistance(distance, tierSpec, sharpness) {
  validateTier(tierSpec);
  if (!Number.isFinite(sharpness) || sharpness <= 0) {
    throw new MetricChronoError("sharpness must be finite and > 0");
  }
  assertDistance(distance);
  const d = distance;
  const gate = sigmoid(sharpness * (d - tierSpec.epsilon));
  const stair = smoothStair(d / tierSpec.delta, sharpness);
  const gain = Math.pow(tierSpec.epsilon / tierSpec.epsilonRef, tierSpec.p);
  return finiteOrMax(gain * gate * stair);
}

export function smoothLadderDistance(distance, ladder, sharpness) {
  validateLadder(ladder);
  return ladder.map((tierSpec) => smoothTickDistance(distance, tierSpec, sharpness));
}

export function normalizeTicks(input, mode = Normalization.None) {
  switch (mode) {
    case Normalization.None:
      return input.slice();
    case Normalization.UnitMax: {
      const maxAbs = input
        .filter((value) => Number.isFinite(value))
        .map((value) => Math.abs(value))
        .reduce((left, right) => Math.max(left, right), 0);
      if (maxAbs <= 0) {
        return input.map(() => 0);
      }
      return input.map((value) => sanitizeSigned(value) / maxAbs);
    }
    case Normalization.Tanh:
      return input.map((value) => Math.tanh(sanitizeSigned(value)));
    default:
      throw new MetricChronoError(`unknown normalization mode: ${mode}`);
  }
}

export function carryRules(epsilons) {
  if (epsilons.length === 0) {
    throw new MetricChronoError("ladder must contain at least one tier");
  }
  return epsilons.map((epsilon, index) => {
    if (!Number.isFinite(epsilon) || epsilon <= 0) {
      throw new MetricChronoError(`invalid tier at index ${index}: epsilon must be finite and > 0`);
    }
    return Math.max(1, Math.ceil(epsilon));
  });
}

export class PromotionCounter {
  constructor(quotas) {
    if (quotas.length === 0) {
      throw new MetricChronoError("ladder must contain at least one tier");
    }
    if (quotas.some((quota) => !Number.isInteger(quota) || quota <= 0)) {
      throw new MetricChronoError("promotion quotas must be positive integers");
    }
    this.quotas = quotas.slice();
    this.counters = quotas.map(() => 0);
  }

  reset() {
    this.counters.fill(0);
  }

  step(eventFlags = undefined) {
    const flags = eventFlags ?? this.quotas.map(() => false);
    if (flags.length !== this.quotas.length) {
      throw new MetricChronoError("event flags length must match quotas");
    }

    const promoted = this.quotas.map(() => false);
    for (let index = 0; index < this.counters.length; index += 1) {
      if (!flags[index]) {
        this.counters[index] += 1;
      }
    }

    let depth = 0;
    while (true) {
      let changed = false;
      for (let index = 0; index < this.counters.length; index += 1) {
        if (this.counters[index] < this.quotas[index]) {
          continue;
        }
        this.counters[index] = 0;
        promoted[index] = true;
        if (index + 1 < this.counters.length) {
          this.counters[index + 1] += 1;
        }
        changed = true;
      }
      if (!changed) {
        break;
      }
      depth += 1;
      if (depth > 1000) {
        throw new MetricChronoError("promotion depth exceeded");
      }
    }

    for (let index = 0; index < flags.length; index += 1) {
      if (flags[index]) {
        this.counters[index] = 0;
      }
    }
    return promoted;
  }
}

export function adaptiveLadderDistance(distance, ladder) {
  validateLadder(ladder);
  assertDistance(distance);
  const d = distance;
  const ticks = Array.from({ length: ladder.length }, () => 0);
  for (let index = 0; index < ladder.length; index += 1) {
    if (d < ladder[index].epsilon) {
      return {
        ticks,
        decision: {
          evaluatedTiers: index + 1,
          firstInactiveTier: index,
          stoppedEarly: true,
        },
      };
    }
    ticks[index] = tickDistance(d, ladder[index]);
  }
  return {
    ticks,
    decision: {
      evaluatedTiers: ladder.length,
      firstInactiveTier: null,
      stoppedEarly: false,
    },
  };
}

export function adaptiveZoomWindow(distance, ladder, radius) {
  validateLadder(ladder);
  assertDistance(distance);
  const d = distance;
  let center = -1;
  for (let index = 0; index < ladder.length; index += 1) {
    if (d >= ladder[index].epsilon) {
      center = index;
    }
  }
  if (center < 0) {
    return null;
  }
  return [Math.max(0, center - radius), Math.min(ladder.length, center + radius + 1)];
}

export class EventLog {
  constructor(tierCount) {
    if (!Number.isInteger(tierCount) || tierCount <= 0) {
      throw new MetricChronoError("tierCount must be a positive integer");
    }
    this.tierCount = tierCount;
    this.records = [];
    this.firstByTier = Array.from({ length: tierCount }, () => null);
    this.lastByTier = Array.from({ length: tierCount }, () => null);
  }

  append(stateId, tickVector) {
    if (tickVector.length !== this.tierCount) {
      throw new MetricChronoError("tick vector length must match tierCount");
    }
    const index = this.records.length;
    this.records.push({
      stateId,
      ticks: tickVector.slice(),
      nextEvent: Array.from({ length: this.tierCount }, () => null),
    });
    for (let tierIndex = 0; tierIndex < this.tierCount; tierIndex += 1) {
      if (sanitizeSigned(tickVector[tierIndex]) <= 0) {
        continue;
      }
      const previous = this.lastByTier[tierIndex];
      if (previous === null) {
        this.firstByTier[tierIndex] = index;
      } else {
        this.records[previous].nextEvent[tierIndex] = index;
      }
      this.lastByTier[tierIndex] = index;
    }
    return index;
  }

  get length() {
    return this.records.length;
  }

  nextEvent(index, tierIndex) {
    if (!Number.isInteger(index) || index < 0 || index >= this.records.length) {
      throw new MetricChronoError(`event index ${index} out of range`);
    }
    if (!Number.isInteger(tierIndex) || tierIndex < 0 || tierIndex >= this.tierCount) {
      throw new MetricChronoError(`tier ${tierIndex} out of range`);
    }
    return this.records[index].nextEvent[tierIndex] ?? null;
  }

  firstEvent(tierIndex) {
    if (!Number.isInteger(tierIndex) || tierIndex < 0 || tierIndex >= this.tierCount) {
      throw new MetricChronoError(`tier ${tierIndex} out of range`);
    }
    return this.firstByTier[tierIndex] ?? null;
  }

  *iterEvents(tierIndex) {
    let index = this.firstEvent(tierIndex);
    while (index !== null) {
      const record = this.records[index];
      yield [index, record];
      index = record.nextEvent[tierIndex];
    }
  }

  compactSummary(tierIndex) {
    return Array.from(this.iterEvents(tierIndex), ([index, record]) => ({
      index,
      stateId: record.stateId,
      tick: record.ticks[tierIndex],
    }));
  }
}

export function weightedConsensus(vectors, weights) {
  if (vectors.length === 0) {
    throw new MetricChronoError("at least one source is required");
  }
  if (vectors.length !== weights.length) {
    throw new MetricChronoError("weights length must match vector count");
  }
  const tiers = vectors[0].length;
  if (tiers === 0) {
    throw new MetricChronoError("vectors must have at least one column");
  }
  const out = Array.from({ length: tiers }, () => 0);
  let totalWeight = 0;
  for (let row = 0; row < vectors.length; row += 1) {
    if (vectors[row].length !== tiers) {
      throw new MetricChronoError("all vectors must have the same length");
    }
    const weight = weights[row];
    if (!Number.isFinite(weight) || weight < 0) {
      throw new MetricChronoError("weights must be finite and >= 0");
    }
    if (weight === 0) {
      continue;
    }
    totalWeight += weight;
    for (let col = 0; col < tiers; col += 1) {
      out[col] += weight * sanitizeSigned(vectors[row][col]);
    }
  }
  if (totalWeight <= 0) {
    throw new MetricChronoError("total consensus weight must be > 0");
  }
  return out.map((value) => value / totalWeight);
}

export function coherenceResidual(sourceTick, consensus) {
  if (sourceTick.length !== consensus.length) {
    throw new MetricChronoError("sourceTick and consensus length must match");
  }
  if (consensus.length === 0) {
    throw new MetricChronoError("consensus must not be empty");
  }
  const mse =
    sourceTick
      .map((value, index) => sanitizeSigned(value) - sanitizeSigned(consensus[index]))
      .map((diff) => diff * diff)
      .reduce((left, right) => left + right, 0) / consensus.length;
  return Math.sqrt(mse);
}

export function coherenceResiduals(vectors, consensus) {
  return vectors.map((vector) => coherenceResidual(vector, consensus));
}

export function simpleWeightUpdate(weights, residuals, learningRate, floor) {
  if (weights.length !== residuals.length) {
    throw new MetricChronoError("weights and residuals length must match");
  }
  if (weights.length === 0) {
    throw new MetricChronoError("at least one weight is required");
  }
  if (!Number.isFinite(learningRate) || learningRate < 0) {
    throw new MetricChronoError("learningRate must be finite and >= 0");
  }
  if (!Number.isFinite(floor) || floor < 0) {
    throw new MetricChronoError("floor must be finite and >= 0");
  }
  const updated = weights.map((weight, index) => {
    const residual = residuals[index];
    if (!Number.isFinite(weight) || weight < 0 || !Number.isFinite(residual) || residual < 0) {
      throw new MetricChronoError("weights and residuals must be finite and >= 0");
    }
    return Math.max(weight * Math.exp(-learningRate * residual), floor);
  });
  const total = updated.reduce((left, right) => left + right, 0);
  if (total <= 0) {
    return updated.map(() => 1 / updated.length);
  }
  return updated.map((value) => value / total);
}

export const OperatingRegime = Object.freeze({
  Quiescent: "quiescent",
  Progress: "progress",
  Churn: "churn",
  Creep: "creep",
});

export class CoverageMeter {
  /**
   * Per-tier streaming coverage meter: a greedy maximal epsilon-packing of
   * the visited image.  Complements tick throughput: coverage counts
   * distinct epsilon-separated territory, is invariant under revisits, and
   * registers sub-threshold relocation (creep) that per-step thresholding
   * is silent on by design.  Storage is pooled: a sample admitted at
   * several tiers is stored once.  NaN distances reject admission.
   */
  constructor(epsilons, metric = euclideanDistance) {
    if (epsilons.length === 0) {
      throw new MetricChronoError("ladder must contain at least one tier");
    }
    if (epsilons.some((epsilon) => !Number.isFinite(epsilon) || epsilon <= 0)) {
      throw new MetricChronoError("coverage epsilons must be finite and positive");
    }
    if (typeof metric !== "function") {
      throw new MetricChronoError("metric must be a distance function");
    }
    this.epsilons = epsilons.slice();
    this.metric = metric;
    this.pool = [];
    this.tierMembers = epsilons.map(() => []);
  }

  get tierCount() {
    return this.epsilons.length;
  }

  observe(state) {
    const distances = new Map();
    const distanceTo = (index) => {
      let value = distances.get(index);
      if (value === undefined) {
        value = this.metric(this.pool[index], state);
        distances.set(index, value);
      }
      return value;
    };
    const admitted = this.epsilons.map((epsilon, tier) => {
      const members = this.tierMembers[tier];
      // newest-first scan: locality-heavy streams short-circuit early
      for (let position = members.length - 1; position >= 0; position -= 1) {
        if (!(distanceTo(members[position]) >= epsilon)) {
          return false;
        }
      }
      return true;
    });
    if (admitted.some(Boolean)) {
      const index = this.pool.length;
      this.pool.push(Array.isArray(state) ? state.slice() : state);
      admitted.forEach((flag, tier) => {
        if (flag) {
          this.tierMembers[tier].push(index);
        }
      });
    }
    return admitted;
  }

  count(tier) {
    const members = this.tierMembers[tier];
    return members === undefined ? undefined : members.length;
  }

  counts() {
    return this.tierMembers.map((members) => members.length);
  }

  uniqueRepresentatives() {
    return this.pool.length;
  }

  representatives(tier) {
    const members = this.tierMembers[tier];
    return members === undefined ? undefined : members.map((index) => this.pool[index]);
  }
}

export function progressEfficiency(coverage, epsilon, pathLength) {
  if (!Number.isFinite(pathLength) || pathLength <= 0) {
    throw new MetricChronoError("path_length must be finite and positive");
  }
  const gained = Math.max(coverage - 1, 0) * epsilon;
  return Math.min(Math.max(gained / pathLength, 0), 1);
}

export function classifyRegime(throughputDelta, coverageDelta) {
  const ticked = throughputDelta > 0;
  const covered = coverageDelta > 0;
  if (ticked && covered) {
    return OperatingRegime.Progress;
  }
  if (ticked) {
    return OperatingRegime.Churn;
  }
  if (covered) {
    return OperatingRegime.Creep;
  }
  return OperatingRegime.Quiescent;
}

export function euclideanDistance(a, b) {
  ensureSameLength(a, b);
  return Math.sqrt(a.map((value, index) => (value - b[index]) ** 2).reduce(sum, 0));
}

export function absoluteDistance(a, b) {
  return Math.abs(a - b);
}

export function tickPair(a, b, metric, tierSpec) {
  return tickDistance(metric(a, b), tierSpec);
}

export function ladderPair(a, b, metric, ladder) {
  return ladderDistance(metric(a, b), ladder);
}

export async function loadWasmMetricChrono(source) {
  const module =
    source instanceof WebAssembly.Module
      ? await WebAssembly.instantiate(source, {})
      : await WebAssembly.instantiateStreaming(fetch(source), {});
  const exports = module.instance ? module.instance.exports : module.exports;
  if (typeof exports.mc_tick_distance_raw !== "function") {
    throw new TypeError("WASM module must export mc_tick_distance_raw");
  }
  return {
    tickDistance(distance, tierSpec) {
      validateTier(tierSpec);
      return exports.mc_tick_distance_raw(
        distance,
        tierSpec.epsilon,
        tierSpec.delta,
        tierSpec.p,
        tierSpec.epsilonRef,
      );
    },
  };
}

function sigmoid(value) {
  const clipped = Math.min(60, Math.max(-60, value));
  return 1 / (1 + Math.exp(-clipped));
}

function smoothStair(x, sharpness) {
  if (x <= 0) {
    return sigmoid(sharpness * x);
  }
  const hard = Math.ceil(x);
  if (!Number.isFinite(hard) || hard > 4096) {
    return hard;
  }
  let out = 1;
  for (let j = 1; j <= hard + 1; j += 1) {
    out += sigmoid(sharpness * (x - j));
  }
  return out;
}

function assertDistance(distance) {
  if (!Number.isFinite(distance) || distance < 0) {
    throw new MetricChronoError("distance must be finite and >= 0");
  }
}

function sanitizeSigned(value) {
  if (Number.isNaN(value)) {
    return 0;
  }
  if (value === Infinity) {
    return Number.MAX_VALUE;
  }
  if (value === -Infinity) {
    return -Number.MAX_VALUE;
  }
  return value;
}

function finiteOrMax(value) {
  if (Number.isNaN(value)) {
    return 0;
  }
  if (!Number.isFinite(value)) {
    return Number.MAX_VALUE;
  }
  return value;
}

function ensureSameLength(a, b) {
  if (a.length !== b.length) {
    throw new MetricChronoError("vectors must have the same length");
  }
}

function ensureSchema(document, expected, fields) {
  ensureExactFields(document, fields, expected);
  if (document.metricchrono_schema !== expected) {
    throw new MetricChronoError(`expected schema ${expected}`);
  }
}

function ensureExactFields(document, fields, context) {
  if (document === null || typeof document !== "object" || Array.isArray(document)) {
    throw new MetricChronoError(`${context} schema must be an object`);
  }
  const allowed = new Set(fields);
  for (const field of Object.keys(document)) {
    if (!allowed.has(field)) {
      throw new MetricChronoError(`unknown field ${field} in ${context} schema`);
    }
  }
  for (const field of fields) {
    if (!Object.hasOwn(document, field)) {
      throw new MetricChronoError(`missing field ${field} in ${context} schema`);
    }
  }
}

function sum(left, right) {
  return left + right;
}
