import { buildCurveLut, DEFAULT_CURVE_POINTS, isIdentityCurve } from "./curves";

export interface CropRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type AspectRatioPreset =
  | "free"
  | "original"
  | "1:1"
  | "16:9"
  | "4:3"
  | "3:2"
  | "4:5"
  | "5:7"
  | "3:5";

export interface SelectiveColorChannel {
  hue: number;
  saturation: number;
  luminance: number;
}

export interface EditParams {
  crop?: CropRect;
  rotate: number;
  straighten: number;
  flipH: boolean;
  flipV: boolean;
  aspectRatio?: AspectRatioPreset;

  perspectiveV: number;
  perspectiveH: number;

  curves?: {
    rgb: Array<[number, number]>;
    r?: Array<[number, number]>;
    g?: Array<[number, number]>;
    b?: Array<[number, number]>;
  };

  levels?: {
    inputBlack: number;
    inputWhite: number;
    gamma: number;
    outputBlack: number;
    outputWhite: number;
  };

  selectiveColor?: {
    reds?: SelectiveColorChannel;
    yellows?: SelectiveColorChannel;
    greens?: SelectiveColorChannel;
    cyans?: SelectiveColorChannel;
    blues?: SelectiveColorChannel;
    magentas?: SelectiveColorChannel;
  };

  brightness: number;
  contrast: number;
  exposure: number;
  highlights: number;
  shadows: number;
  brilliance: number;
  blackPoint: number;

  saturation: number;
  vibrance: number;
  warmth: number;
  tint: number;

  sharpness: number;
  definition: number;
  noiseReduction: number;

  vignette: number;
  vignetteRadius: number;
  grain: number;

  bwIntensity: number;
  bwTone: number;
}

export const DEFAULT_LEVELS = {
  inputBlack: 0,
  inputWhite: 255,
  gamma: 1.0,
  outputBlack: 0,
  outputWhite: 255,
};

export const DEFAULT_EDIT_PARAMS: EditParams = {
  rotate: 0,
  straighten: 0,
  flipH: false,
  flipV: false,
  perspectiveV: 0,
  perspectiveH: 0,
  brightness: 0,
  contrast: 0,
  exposure: 0,
  highlights: 0,
  shadows: 0,
  brilliance: 0,
  blackPoint: 0,
  saturation: 0,
  vibrance: 0,
  warmth: 0,
  tint: 0,
  sharpness: 0,
  definition: 0,
  noiseReduction: 0,
  vignette: 0,
  vignetteRadius: 50,
  grain: 0,
  bwIntensity: 0,
  bwTone: 0,
};

export function parseEditParams(json: string | null | undefined): EditParams {
  if (!json?.trim()) return { ...DEFAULT_EDIT_PARAMS };
  try {
    return { ...DEFAULT_EDIT_PARAMS, ...JSON.parse(json) };
  } catch {
    return { ...DEFAULT_EDIT_PARAMS };
  }
}

export function serializeEditParams(params: EditParams): string {
  return JSON.stringify(params);
}

function isDefaultLevels(levels: EditParams["levels"]): boolean {
  if (!levels) return true;
  const d = DEFAULT_LEVELS;
  return (
    levels.inputBlack === d.inputBlack &&
    levels.inputWhite === d.inputWhite &&
    levels.gamma === d.gamma &&
    levels.outputBlack === d.outputBlack &&
    levels.outputWhite === d.outputWhite
  );
}

function isDefaultSelectiveColor(sc: EditParams["selectiveColor"]): boolean {
  if (!sc) return true;
  const channels = [sc.reds, sc.yellows, sc.greens, sc.cyans, sc.blues, sc.magentas];
  return channels.every(
    (c) => !c || (c.hue === 0 && c.saturation === 0 && c.luminance === 0),
  );
}

export function isDefaultEditParams(params: EditParams): boolean {
  const d = DEFAULT_EDIT_PARAMS;
  // Lazy import check for curves - inline identity check
  const curvesDefault =
    !params.curves ||
    (isIdentityCurvesChannel(params.curves.rgb) &&
      (!params.curves.r || isIdentityCurvesChannel(params.curves.r)) &&
      (!params.curves.g || isIdentityCurvesChannel(params.curves.g)) &&
      (!params.curves.b || isIdentityCurvesChannel(params.curves.b)));

  return (
    !params.crop &&
    params.rotate === d.rotate &&
    params.straighten === d.straighten &&
    !params.flipH &&
    !params.flipV &&
    params.perspectiveV === 0 &&
    params.perspectiveH === 0 &&
    curvesDefault &&
    isDefaultLevels(params.levels) &&
    isDefaultSelectiveColor(params.selectiveColor) &&
    params.brightness === 0 &&
    params.contrast === 0 &&
    params.exposure === 0 &&
    params.highlights === 0 &&
    params.shadows === 0 &&
    params.brilliance === 0 &&
    params.blackPoint === 0 &&
    params.saturation === 0 &&
    params.vibrance === 0 &&
    params.warmth === 0 &&
    params.tint === 0 &&
    params.sharpness === 0 &&
    params.definition === 0 &&
    params.noiseReduction === 0 &&
    params.vignette === 0 &&
    params.vignetteRadius === 50 &&
    params.grain === 0 &&
    params.bwIntensity === 0 &&
    params.bwTone === 0
  );
}

function isIdentityCurvesChannel(points: Array<[number, number]> | undefined): boolean {
  return isIdentityCurve(points);
}

export function buildCssFilter(params: EditParams): string {
  let brightness = 1 + params.brightness / 100 + params.exposure / 200;
  let contrast = 1 + params.contrast / 100 + params.definition / 200;

  // Approximate levels/curves with brightness/contrast for preview
  if (params.levels && !isDefaultLevels(params.levels)) {
    const { inputBlack, inputWhite, gamma, outputBlack, outputWhite } = params.levels;
    const inRange = Math.max(1, inputWhite - inputBlack);
    const outRange = outputWhite - outputBlack;
    contrast *= (255 / inRange) * (outRange / 255);
    brightness *= (outputWhite / 255) * (1 + (1 - gamma) * 0.2);
    brightness += (outputBlack - inputBlack) / 512;
  }

  if (params.curves?.rgb && !isIdentityCurve(params.curves.rgb)) {
    const lut = buildCurveLut(params.curves.rgb);
    const midIn = 128;
    const midOut = lut[midIn];
    contrast *= 1 + (midOut - midIn) / 256;
    brightness *= 1 + (lut[255] - lut[0]) / 512;
  }
  const saturate = 1 + params.saturation / 100 + params.vibrance / 150;
  const sepia = params.bwIntensity / 100;
  const warmthHue = params.warmth * 0.3 + params.tint * -0.2;
  const parts = [
    `brightness(${brightness.toFixed(3)})`,
    `contrast(${contrast.toFixed(3)})`,
    `saturate(${Math.max(0, saturate).toFixed(3)})`,
  ];
  if (sepia > 0) parts.push(`sepia(${sepia.toFixed(3)})`);
  if (Math.abs(warmthHue) > 0.01) parts.push(`hue-rotate(${warmthHue.toFixed(1)}deg)`);
  if (params.brilliance !== 0) {
    parts.push(`brightness(${(1 + params.brilliance / 300).toFixed(3)})`);
  }
  return parts.join(" ");
}

export function buildImageTransform(params: EditParams): string {
  const transforms: string[] = [];
  const hasPerspective = params.perspectiveV !== 0 || params.perspectiveH !== 0;
  if (hasPerspective) {
    const rotX = params.perspectiveV * 0.15;
    const rotY = params.perspectiveH * 0.15;
    transforms.push(`perspective(800px) rotateX(${rotX}deg) rotateY(${rotY}deg)`);
  }
  const totalRotate = params.rotate + params.straighten;
  if (totalRotate !== 0) transforms.push(`rotate(${totalRotate}deg)`);
  const scaleX = params.flipH ? -1 : 1;
  const scaleY = params.flipV ? -1 : 1;
  if (scaleX !== 1 || scaleY !== 1) transforms.push(`scale(${scaleX}, ${scaleY})`);
  return transforms.join(" ");
}

export { DEFAULT_CURVE_POINTS };

export function aspectRatioValue(preset: AspectRatioPreset, originalRatio: number): number | null {
  switch (preset) {
    case "free":
      return null;
    case "original":
      return originalRatio;
    case "1:1":
      return 1;
    case "16:9":
      return 16 / 9;
    case "4:3":
      return 4 / 3;
    case "3:2":
      return 3 / 2;
    case "4:5":
      return 4 / 5;
    case "5:7":
      return 5 / 7;
    case "3:5":
      return 3 / 5;
    default:
      return null;
  }
}

export function buildClipPath(crop: CropRect | undefined): string | undefined {
  if (!crop || crop.width <= 0 || crop.height <= 0) return undefined;
  const left = crop.x * 100;
  const top = crop.y * 100;
  const right = (1 - crop.x - crop.width) * 100;
  const bottom = (1 - crop.y - crop.height) * 100;
  return `inset(${top}% ${right}% ${bottom}% ${left}%)`;
}

export function formatSliderValue(value: number): string {
  if (Number.isInteger(value)) return String(value);
  return value.toFixed(1);
}
