export function tier(epsilon, delta, p = 0.5, epsilonRef = 1.0) {
  return { epsilon, delta, p, epsilonRef };
}

export function tickDistance(distance, tierSpec) {
  const d = sanitizeDistance(distance);
  if (d < tierSpec.epsilon) {
    return 0;
  }
  const gain = Math.pow(tierSpec.epsilon / tierSpec.epsilonRef, tierSpec.p);
  return finiteOrMax(gain * Math.ceil(d / tierSpec.delta));
}

export function ladderDistance(distance, ladder) {
  return ladder.map((tierSpec) => tickDistance(distance, tierSpec));
}

export function geometricLadder(epsilon0, delta0, ratio, tiers, p = 0.5, epsilonRef = 1.0) {
  if (tiers <= 0) {
    throw new RangeError("tiers must be > 0");
  }
  if (!Number.isFinite(ratio) || ratio <= 1) {
    throw new RangeError("ratio must be finite and > 1");
  }
  return Array.from({ length: tiers }, (_, index) => {
    const scale = Math.pow(ratio, index);
    return tier(epsilon0 * scale, delta0 * scale, p, epsilonRef);
  });
}

export function weightedConsensus(vectors, weights) {
  if (vectors.length === 0) {
    throw new RangeError("at least one source is required");
  }
  if (vectors.length !== weights.length) {
    throw new RangeError("weights length must match vector count");
  }
  const tiers = vectors[0].length;
  const out = Array.from({ length: tiers }, () => 0);
  let totalWeight = 0;
  for (let row = 0; row < vectors.length; row += 1) {
    if (vectors[row].length !== tiers) {
      throw new RangeError("all vectors must have the same length");
    }
    const weight = weights[row];
    if (!Number.isFinite(weight) || weight < 0) {
      throw new RangeError("weights must be finite and >= 0");
    }
    totalWeight += weight;
    for (let col = 0; col < tiers; col += 1) {
      out[col] += weight * sanitizeSigned(vectors[row][col]);
    }
  }
  if (totalWeight <= 0) {
    throw new RangeError("total consensus weight must be > 0");
  }
  return out.map((value) => value / totalWeight);
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

function sanitizeDistance(distance) {
  if (Number.isNaN(distance) || distance < 0) {
    return 0;
  }
  if (distance === Infinity) {
    return Number.MAX_VALUE;
  }
  return distance;
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
