import assert from "node:assert/strict";

const moduleUrl = new URL("../src/index.js", import.meta.url);
const mc = await import(moduleUrl.href);

const ladder = mc.geometricLadder(0.03, 0.1, 3.0, 3, 0.0, 1.0);
assert.deepEqual(mc.ladderDistance(1.0, ladder), [10, 4, 2]);
assert.equal(typeof mc.loadWasmMetricChrono, "function");
