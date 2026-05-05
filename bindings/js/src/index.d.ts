export class MetricChronoError extends Error {}

export interface Tier {
  epsilon: number;
  delta: number;
  p: number;
  epsilonRef: number;
}

export interface ZoomDecision {
  evaluatedTiers: number;
  firstInactiveTier: number | null;
  stoppedEarly: boolean;
}

export interface AdaptiveLadderResult {
  ticks: number[];
  decision: ZoomDecision;
}

export interface EventRecord<T = unknown> {
  stateId: T;
  ticks: number[];
  nextEvent: Array<number | null>;
}

export interface EventSummary<T = unknown> {
  index: number;
  stateId: T;
  tick: number;
}

export const Normalization: {
  readonly None: "none";
  readonly UnitMax: "unitMax";
  readonly Tanh: "tanh";
};

export function tier(epsilon: number, delta: number, p?: number, epsilonRef?: number): Tier;
export function validateTier(tierSpec: Tier, index?: number): void;
export function validateLadder(ladder: readonly Tier[]): void;
export function tickDistance(distance: number, tierSpec: Tier): number;
export function ladderDistance(distance: number, ladder: readonly Tier[]): number[];
export function customLadder(tiers: readonly Tier[]): Tier[];
export function geometricLadder(
  epsilon0: number,
  delta0: number,
  ratio: number,
  tiers: number,
  p?: number,
  epsilonRef?: number,
): Tier[];
export function tierFromSchema(document: unknown): Tier;
export function tierToSchema(tierSpec: Tier): {
  metricchrono_schema: "tier.v1";
  epsilon: number;
  delta: number;
  p: number;
  epsilon_ref: number;
};
export function ladderFromSchema(document: unknown): Tier[];
export function ladderToSchema(ladder: readonly Tier[]): {
  metricchrono_schema: "ladder.v1";
  tiers: Array<{ epsilon: number; delta: number; p: number; epsilon_ref: number }>;
};
export function tickVectorFromSchema(document: unknown): number[];
export function tickVectorToSchema(ticks: readonly number[]): {
  metricchrono_schema: "tick_vector.v1";
  ticks: number[];
};
export function consensusResultFromSchema(document: unknown): {
  metricchrono_schema: "consensus_result.v1";
  consensus: number[];
  residuals: number[];
  weights: number[];
};
export function smoothTickDistance(distance: number, tierSpec: Tier, sharpness: number): number;
export function smoothLadderDistance(
  distance: number,
  ladder: readonly Tier[],
  sharpness: number,
): number[];
export function normalizeTicks(
  input: readonly number[],
  mode?: "none" | "unitMax" | "tanh",
): number[];
export function carryRules(epsilons: readonly number[]): number[];

export class PromotionCounter {
  readonly quotas: number[];
  readonly counters: number[];
  constructor(quotas: readonly number[]);
  reset(): void;
  step(eventFlags?: readonly boolean[]): boolean[];
}

export function adaptiveLadderDistance(
  distance: number,
  ladder: readonly Tier[],
): AdaptiveLadderResult;
export function adaptiveZoomWindow(
  distance: number,
  ladder: readonly Tier[],
  radius: number,
): [number, number] | null;

export class EventLog<T = unknown> {
  readonly tierCount: number;
  readonly records: Array<EventRecord<T>>;
  constructor(tierCount: number);
  append(stateId: T, tickVector: readonly number[]): number;
  readonly length: number;
  nextEvent(index: number, tierIndex: number): number | null;
  firstEvent(tierIndex: number): number | null;
  iterEvents(tierIndex: number): IterableIterator<[number, EventRecord<T>]>;
  compactSummary(tierIndex: number): Array<EventSummary<T>>;
}

export function weightedConsensus(
  vectors: readonly (readonly number[])[],
  weights: readonly number[],
): number[];
export function coherenceResidual(sourceTick: readonly number[], consensus: readonly number[]): number;
export function coherenceResiduals(
  vectors: readonly (readonly number[])[],
  consensus: readonly number[],
): number[];
export function simpleWeightUpdate(
  weights: readonly number[],
  residuals: readonly number[],
  learningRate: number,
  floor: number,
): number[];

export function euclideanDistance(a: readonly number[], b: readonly number[]): number;
export function squaredEuclideanDistance(a: readonly number[], b: readonly number[]): number;
export function absoluteDistance(a: number, b: number): number;
export function manhattanDistance(a: readonly number[], b: readonly number[]): number;
export function cosineDistance(a: readonly number[], b: readonly number[]): number;
export function kullbackLeiblerDistance(
  a: readonly number[],
  b: readonly number[],
  epsilon?: number,
): number;
export function jensenShannonDistance(
  a: readonly number[],
  b: readonly number[],
  epsilon?: number,
): number;
export function diagonalMahalanobisDistance(
  a: readonly number[],
  b: readonly number[],
  inverseVariance: readonly number[],
): number;
export function tickPair<T>(
  a: T,
  b: T,
  metric: (a: T, b: T) => number,
  tierSpec: Tier,
): number;
export function ladderPair<T>(
  a: T,
  b: T,
  metric: (a: T, b: T) => number,
  ladder: readonly Tier[],
): number[];

export function loadWasmMetricChrono(
  source: WebAssembly.Module | string | URL,
): Promise<{
  tickDistance(distance: number, tierSpec: Tier): number;
}>;
