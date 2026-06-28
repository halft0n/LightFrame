import { describe, it, expect } from "vitest";
import {
  DEFAULT_EDIT_PARAMS,
  parseEditParams,
  serializeEditParams,
  isDefaultEditParams,
  buildCssFilter,
  buildImageTransform,
  buildClipPath,
  aspectRatioValue,
  formatSliderValue,
  type EditParams,
} from "./editParams";

describe("parseEditParams", () => {
  it("should return defaults for null input", () => {
    expect(parseEditParams(null)).toEqual(DEFAULT_EDIT_PARAMS);
  });

  it("should return defaults for empty string", () => {
    expect(parseEditParams("")).toEqual(DEFAULT_EDIT_PARAMS);
    expect(parseEditParams("   ")).toEqual(DEFAULT_EDIT_PARAMS);
  });

  it("should return defaults for invalid JSON", () => {
    expect(parseEditParams("{not valid json")).toEqual(DEFAULT_EDIT_PARAMS);
  });

  it("should parse valid JSON and merge with defaults", () => {
    const json = JSON.stringify({ brightness: 25, contrast: -10, rotate: 90 });
    const result = parseEditParams(json);
    expect(result.brightness).toBe(25);
    expect(result.contrast).toBe(-10);
    expect(result.rotate).toBe(90);
    expect(result.flipH).toBe(false);
    expect(result.vignetteRadius).toBe(50);
  });

  it("should handle partial params (only brightness set)", () => {
    const result = parseEditParams(JSON.stringify({ brightness: 42 }));
    expect(result.brightness).toBe(42);
    expect(result.contrast).toBe(0);
    expect(result.rotate).toBe(0);
  });

  it("should deep merge levels and preserve nested curves/selectiveColor objects", () => {
    const json = JSON.stringify({
      curves: { rgb: [[0, 0], [255, 255]], r: [[0, 0], [255, 255]] },
      levels: { inputBlack: 20 },
      selectiveColor: { reds: { hue: 10, saturation: 0, luminance: 0 } },
    });
    const result = parseEditParams(json);

    expect(result.curves?.rgb).toEqual([[0, 0], [255, 255]]);
    expect(result.curves?.r).toEqual([[0, 0], [255, 255]]);
    expect(result.levels).toEqual({
      inputBlack: 20,
      inputWhite: 255,
      gamma: 1.0,
      outputBlack: 0,
      outputWhite: 255,
    });
    expect(result.selectiveColor?.reds).toEqual({ hue: 10, saturation: 0, luminance: 0 });
  });

  it("should merge partial levels JSON with defaults", () => {
    const result = parseEditParams(JSON.stringify({ levels: { gamma: 1.5, outputWhite: 240 } }));
    expect(result.levels).toEqual({
      inputBlack: 0,
      inputWhite: 255,
      gamma: 1.5,
      outputBlack: 0,
      outputWhite: 240,
    });
  });

  it("should preserve partial selectiveColor JSON as provided", () => {
    const result = parseEditParams(
      JSON.stringify({
        selectiveColor: {
          blues: { saturation: -15 },
          greens: { hue: 5, luminance: 10 },
        },
      }),
    );
    expect(result.selectiveColor?.blues).toEqual({ saturation: -15 });
    expect(result.selectiveColor?.greens).toEqual({ hue: 5, luminance: 10 });
    expect(result.selectiveColor?.reds).toBeUndefined();
  });
});

describe("serializeEditParams", () => {
  it("should serialize to JSON string", () => {
    const params: EditParams = { ...DEFAULT_EDIT_PARAMS, brightness: 10 };
    expect(serializeEditParams(params)).toBe(JSON.stringify(params));
  });

  it("should be reversible with parseEditParams", () => {
    const params: EditParams = {
      ...DEFAULT_EDIT_PARAMS,
      brightness: 15,
      rotate: 45,
      flipH: true,
    };
    const roundTripped = parseEditParams(serializeEditParams(params));
    expect(roundTripped).toEqual(params);
  });
});

describe("isDefaultEditParams", () => {
  it("should return true for default params", () => {
    expect(isDefaultEditParams({ ...DEFAULT_EDIT_PARAMS })).toBe(true);
  });

  it("should return false when brightness is non-zero", () => {
    expect(isDefaultEditParams({ ...DEFAULT_EDIT_PARAMS, brightness: 5 })).toBe(false);
  });

  it("should return false when crop is set", () => {
    expect(
      isDefaultEditParams({
        ...DEFAULT_EDIT_PARAMS,
        crop: { x: 0.1, y: 0.1, width: 0.8, height: 0.8 },
      }),
    ).toBe(false);
  });

  it("should return false when flipH is true", () => {
    expect(isDefaultEditParams({ ...DEFAULT_EDIT_PARAMS, flipH: true })).toBe(false);
  });

  it("should return false when curves are set", () => {
    expect(
      isDefaultEditParams({
        ...DEFAULT_EDIT_PARAMS,
        curves: { rgb: [[0, 0], [128, 200], [255, 255]] },
      }),
    ).toBe(false);
  });

  it("should return false when levels are set", () => {
    expect(
      isDefaultEditParams({
        ...DEFAULT_EDIT_PARAMS,
        levels: {
          inputBlack: 10,
          inputWhite: 255,
          gamma: 1.0,
          outputBlack: 0,
          outputWhite: 255,
        },
      }),
    ).toBe(false);
  });

  it("should return false when selectiveColor is set", () => {
    expect(
      isDefaultEditParams({
        ...DEFAULT_EDIT_PARAMS,
        selectiveColor: { reds: { hue: 5, saturation: 0, luminance: 0 } },
      }),
    ).toBe(false);
  });

  it("should return false when perspectiveV is non-zero", () => {
    expect(isDefaultEditParams({ ...DEFAULT_EDIT_PARAMS, perspectiveV: 10 })).toBe(false);
  });

  it("should return false for non-default curves rgb channel", () => {
    expect(
      isDefaultEditParams({
        ...DEFAULT_EDIT_PARAMS,
        curves: { rgb: [[0, 0], [128, 200], [255, 255]] },
      }),
    ).toBe(false);
  });

  it("should return true for identity curves", () => {
    expect(
      isDefaultEditParams({
        ...DEFAULT_EDIT_PARAMS,
        curves: { rgb: [[0, 0], [128, 128], [255, 255]] },
      }),
    ).toBe(true);
  });

  it("should return false for non-default levels inputWhite", () => {
    expect(
      isDefaultEditParams({
        ...DEFAULT_EDIT_PARAMS,
        levels: { inputBlack: 0, inputWhite: 240, gamma: 1.0, outputBlack: 0, outputWhite: 255 },
      }),
    ).toBe(false);
  });

  it("should return false for non-default selectiveColor channel", () => {
    expect(
      isDefaultEditParams({
        ...DEFAULT_EDIT_PARAMS,
        selectiveColor: { cyans: { hue: 0, saturation: 3, luminance: 0 } },
      }),
    ).toBe(false);
  });
});

describe("buildCssFilter", () => {
  it("should return valid CSS filter for defaults", () => {
    const filter = buildCssFilter(DEFAULT_EDIT_PARAMS);
    expect(filter).toContain("brightness(1.000)");
    expect(filter).toContain("contrast(1.000)");
    expect(filter).toContain("saturate(1.000)");
  });

  it("should increase brightness in filter when brightness > 0", () => {
    const filter = buildCssFilter({ ...DEFAULT_EDIT_PARAMS, brightness: 50 });
    expect(filter).toContain("brightness(1.500)");
  });

  it("should add sepia for B&W intensity", () => {
    const filter = buildCssFilter({ ...DEFAULT_EDIT_PARAMS, bwIntensity: 50 });
    expect(filter).toContain("sepia(0.500)");
  });

  it("should add hue-rotate for warmth", () => {
    const filter = buildCssFilter({ ...DEFAULT_EDIT_PARAMS, warmth: 10 });
    expect(filter).toContain("hue-rotate(3.0deg)");
  });

  it("should adjust filter when levels are non-default", () => {
    const defaultFilter = buildCssFilter(DEFAULT_EDIT_PARAMS);
    const levelsFilter = buildCssFilter({
      ...DEFAULT_EDIT_PARAMS,
      levels: {
        inputBlack: 10,
        inputWhite: 245,
        gamma: 1.2,
        outputBlack: 5,
        outputWhite: 250,
      },
    });
    expect(levelsFilter).not.toBe(defaultFilter);
    expect(levelsFilter).toMatch(/brightness\([\d.]+\)/);
    expect(levelsFilter).toMatch(/contrast\([\d.]+\)/);
  });

  it("should adjust filter when curves rgb is non-identity", () => {
    const defaultFilter = buildCssFilter(DEFAULT_EDIT_PARAMS);
    const curvesFilter = buildCssFilter({
      ...DEFAULT_EDIT_PARAMS,
      curves: { rgb: [[0, 0], [128, 180], [255, 255]] },
    });
    expect(curvesFilter).not.toBe(defaultFilter);
    expect(curvesFilter).toMatch(/contrast\([\d.]+\)/);
  });
});

describe("buildImageTransform", () => {
  it("should return empty string for defaults (no rotation/flip)", () => {
    expect(buildImageTransform(DEFAULT_EDIT_PARAMS)).toBe("");
  });

  it("should include rotate for non-zero rotation", () => {
    expect(buildImageTransform({ ...DEFAULT_EDIT_PARAMS, rotate: 90 })).toContain("rotate(90deg)");
  });

  it("should include scale for flip", () => {
    expect(buildImageTransform({ ...DEFAULT_EDIT_PARAMS, flipH: true })).toContain("scale(-1, 1)");
  });

  it("should include perspective for non-zero perspectiveV", () => {
    const transform = buildImageTransform({ ...DEFAULT_EDIT_PARAMS, perspectiveV: 10 });
    expect(transform).toContain("perspective(800px)");
    expect(transform).toContain("rotateX(1.5deg)");
  });

  it("should combine rotation and perspective", () => {
    const transform = buildImageTransform({
      ...DEFAULT_EDIT_PARAMS,
      rotate: 45,
      perspectiveV: 10,
    });
    expect(transform).toContain("rotate(45deg)");
    expect(transform).toContain("perspective(800px)");
  });
});

describe("buildClipPath", () => {
  it("should return undefined for no crop", () => {
    expect(buildClipPath(undefined)).toBeUndefined();
  });

  it("should return undefined for zero-size crop", () => {
    expect(buildClipPath({ x: 0, y: 0, width: 0, height: 0.5 })).toBeUndefined();
    expect(buildClipPath({ x: 0, y: 0, width: 0.5, height: 0 })).toBeUndefined();
  });

  it("should return valid inset for normal crop", () => {
    const clip = buildClipPath({ x: 0.1, y: 0.2, width: 0.5, height: 0.4 });
    expect(clip).toBe("inset(20% 40% 40% 10%)");
  });

  it("should return undefined for negative width or height", () => {
    expect(buildClipPath({ x: 0, y: 0, width: -0.1, height: 0.5 })).toBeUndefined();
    expect(buildClipPath({ x: 0, y: 0, width: 0.5, height: -0.2 })).toBeUndefined();
  });

  it("should handle edge crop at image bounds", () => {
    const clip = buildClipPath({ x: 0, y: 0, width: 1, height: 1 });
    expect(clip).toBe("inset(0% 0% 0% 0%)");
  });
});

describe("aspectRatioValue", () => {
  it("should return null for free", () => {
    expect(aspectRatioValue("free", 1.5)).toBeNull();
  });

  it("should return original ratio for original", () => {
    expect(aspectRatioValue("original", 1.5)).toBe(1.5);
  });

  it("should return 1 for 1:1", () => {
    expect(aspectRatioValue("1:1", 1.5)).toBe(1);
  });

  it("should return 16/9 for 16:9", () => {
    expect(aspectRatioValue("16:9", 1.5)).toBeCloseTo(16 / 9);
  });

  it("should return correct ratios for all presets", () => {
    expect(aspectRatioValue("4:3", 2)).toBeCloseTo(4 / 3);
    expect(aspectRatioValue("3:2", 2)).toBeCloseTo(3 / 2);
    expect(aspectRatioValue("4:5", 2)).toBeCloseTo(4 / 5);
    expect(aspectRatioValue("5:7", 2)).toBeCloseTo(5 / 7);
    expect(aspectRatioValue("3:5", 2)).toBeCloseTo(3 / 5);
  });
});

describe("formatSliderValue", () => {
  it("should format integer as string", () => {
    expect(formatSliderValue(42)).toBe("42");
    expect(formatSliderValue(0)).toBe("0");
  });

  it("should format float with one decimal", () => {
    expect(formatSliderValue(3.14)).toBe("3.1");
    expect(formatSliderValue(-2.56)).toBe("-2.6");
  });

  it("should distinguish integer from decimal values", () => {
    expect(formatSliderValue(10)).toBe("10");
    expect(formatSliderValue(10.0)).toBe("10");
    expect(formatSliderValue(10.4)).toBe("10.4");
    expect(Number.isInteger(10.0)).toBe(true);
    expect(Number.isInteger(10.4)).toBe(false);
  });
});
