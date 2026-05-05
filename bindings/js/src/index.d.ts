export interface Tier {
  epsilon: number;
  delta: number;
  p: number;
  epsilonRef: number;
}

export function tier(epsilon: number, delta: number, p?: number, epsilonRef?: number): Tier;
export function tickDistance(distance: number, tierSpec: Tier): number;
export function ladderDistance(distance: number, ladder: readonly Tier[]): number[];
export function geometricLadder(
  epsilon0: number,
  delta0: number,
  ratio: number,
  tiers: number,
  p?: number,
  epsilonRef?: number,
): Tier[];
export function weightedConsensus(
  vectors: readonly (readonly number[])[],
  weights: readonly number[],
): number[];
export function loadWasmMetricChrono(
  source: WebAssembly.Module | string | URL,
): Promise<{
  tickDistance(distance: number, tierSpec: Tier): number;
}>;
