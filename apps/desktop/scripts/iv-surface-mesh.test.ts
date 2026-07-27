import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  buildIvSurfaceMesh,
  ivColorRgba,
  sampleMeshCell,
} from "../src/lib/iv-surface/mesh.ts";

describe("buildIvSurfaceMesh", () => {
  it("builds a dense term×strike grid preferring calls", () => {
    const mesh = buildIvSurfaceMesh([
      { expiration: "2026-09-18", strike: 220, optionType: "call", impliedVolatility: 0.25 },
      { expiration: "2026-08-21", strike: 210, optionType: "call", impliedVolatility: 0.28 },
      { expiration: "2026-08-21", strike: 220, optionType: "put", impliedVolatility: 0.3 },
      { expiration: "2026-08-21", strike: 220, optionType: "call", impliedVolatility: 0.26 },
      { expiration: "2026-09-18", strike: 210, optionType: "call", impliedVolatility: 0.27 },
    ]);

    assert.deepEqual(mesh.terms, ["2026-08-21", "2026-09-18"]);
    assert.deepEqual(mesh.strikes, [210, 220]);
    assert.equal(mesh.values[0][0], 0.28);
    assert.equal(mesh.values[0][1], 0.26);
    assert.equal(mesh.values[1][0], 0.27);
    assert.equal(mesh.values[1][1], 0.25);
    assert.equal(mesh.minIv, 0.25);
    assert.equal(mesh.maxIv, 0.28);
  });

  it("leaves holes null when a cell is missing", () => {
    const mesh = buildIvSurfaceMesh([
      { expiration: "2026-08-21", strike: 210, optionType: "call", impliedVolatility: 0.2 },
      { expiration: "2026-09-18", strike: 220, optionType: "call", impliedVolatility: 0.3 },
    ]);
    assert.equal(mesh.values[0][1], null);
    assert.equal(mesh.values[1][0], null);
    assert.equal(sampleMeshCell(mesh, 0, 0), 0.2);
    assert.equal(sampleMeshCell(mesh, 1, 1), 0.3);
  });
});

describe("ivColorRgba", () => {
  it("maps low IV toward cool and high IV toward warm brand tones", () => {
    const low = ivColorRgba(0, 0, 1);
    const high = ivColorRgba(1, 0, 1);
    assert.ok(low[2] > low[0], "low IV leans blue");
    assert.ok(high[0] > high[2], "high IV leans warm");
    assert.equal(low[3], 255);
  });
});
