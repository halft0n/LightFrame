import { describe, it, expect } from "vitest";
import { buildCurveLut, DEFAULT_CURVE_POINTS } from "./curves";

describe("Curve interpolation", () => {
  it("should produce identity LUT for default points [[0,0],[255,255]]", () => {
    const lut = buildCurveLut([
      [0, 0],
      [255, 255],
    ]);
    for (let i = 0; i < 256; i++) {
      expect(lut[i]).toBe(i);
    }

    const defaultLut = buildCurveLut(DEFAULT_CURVE_POINTS);
    for (let i = 0; i < 256; i++) {
      expect(defaultLut[i]).toBe(i);
    }
  });

  it("should clamp values to 0-255 range", () => {
    const lut = buildCurveLut([
      [0, -50],
      [128, 300],
      [255, 400],
    ]);
    for (let i = 0; i < 256; i++) {
      expect(lut[i]).toBeGreaterThanOrEqual(0);
      expect(lut[i]).toBeLessThanOrEqual(255);
    }
    expect(lut[0]).toBe(0);
    expect(lut[255]).toBe(255);
  });

  it("should handle single control point (edge case)", () => {
    const lut = buildCurveLut([[128, 200]]);
    for (let i = 0; i < 256; i++) {
      expect(lut[i]).toBe(200);
    }
  });

  it("should produce monotonic output for monotonic input", () => {
    const lut = buildCurveLut([
      [0, 0],
      [64, 40],
      [128, 100],
      [192, 180],
      [255, 255],
    ]);
    for (let i = 1; i < 256; i++) {
      expect(lut[i]).toBeGreaterThanOrEqual(lut[i - 1]);
    }
  });

  it("should handle custom three-point curve", () => {
    const lut = buildCurveLut([
      [0, 0],
      [128, 64],
      [255, 255],
    ]);
    expect(lut[0]).toBe(0);
    expect(lut[255]).toBe(255);
    expect(lut[128]).toBeCloseTo(64, 0);
    expect(lut[64]).toBeLessThan(64);
    expect(lut[192]).toBeGreaterThan(128);
  });
});
