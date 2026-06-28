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

export interface EditParams {
  crop?: CropRect;
  rotate: number;
  straighten: number;
  flipH: boolean;
  flipV: boolean;
  aspectRatio?: AspectRatioPreset;

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

export const DEFAULT_EDIT_PARAMS: EditParams = {
  rotate: 0,
  straighten: 0,
  flipH: false,
  flipV: false,
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

export function isDefaultEditParams(params: EditParams): boolean {
  const d = DEFAULT_EDIT_PARAMS;
  return (
    !params.crop &&
    params.rotate === d.rotate &&
    params.straighten === d.straighten &&
    !params.flipH &&
    !params.flipV &&
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

export function buildCssFilter(params: EditParams): string {
  const brightness = 1 + params.brightness / 100 + params.exposure / 200;
  const contrast = 1 + params.contrast / 100 + params.definition / 200;
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
  const totalRotate = params.rotate + params.straighten;
  if (totalRotate !== 0) transforms.push(`rotate(${totalRotate}deg)`);
  const scaleX = params.flipH ? -1 : 1;
  const scaleY = params.flipV ? -1 : 1;
  if (scaleX !== 1 || scaleY !== 1) transforms.push(`scale(${scaleX}, ${scaleY})`);
  return transforms.join(" ");
}

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
