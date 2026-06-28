export type CurvePoint = [number, number];

export const DEFAULT_CURVE_POINTS: CurvePoint[] = [
  [0, 0],
  [128, 128],
  [255, 255],
];

export const DEFAULT_CURVES = {
  rgb: [...DEFAULT_CURVE_POINTS] as CurvePoint[],
};

export function sortCurvePoints(points: CurvePoint[]): CurvePoint[] {
  return [...points].sort((a, b) => a[0] - b[0]);
}

/** Monotone cubic interpolation (Fritsch-Carlson) through control points. */
export function buildCurveLut(points: CurvePoint[]): Uint8Array {
  const sorted = sortCurvePoints(points);
  const lut = new Uint8Array(256);

  if (sorted.length === 0) {
    for (let i = 0; i < 256; i++) lut[i] = i;
    return lut;
  }
  if (sorted.length === 1) {
    const v = clamp(Math.round(sorted[0][1]), 0, 255);
    lut.fill(v);
    return lut;
  }

  const xs = sorted.map((p) => p[0]);
  const ys = sorted.map((p) => clamp(p[1], 0, 255));

  const n = xs.length;
  const ms = new Float64Array(n);
  for (let i = 0; i < n - 1; i++) {
    const dx = xs[i + 1] - xs[i];
    ms[i] = dx !== 0 ? (ys[i + 1] - ys[i]) / dx : 0;
  }
  ms[n - 1] = ms[n - 2] ?? 0;

  const tangents = new Float64Array(n);
  tangents[0] = ms[0];
  tangents[n - 1] = ms[n - 2];
  for (let i = 1; i < n - 1; i++) {
    if (ms[i - 1] * ms[i] <= 0) {
      tangents[i] = 0;
    } else {
      tangents[i] = (ms[i - 1] + ms[i]) / 2;
    }
  }

  for (let x = 0; x < 256; x++) {
    let seg = 0;
    while (seg < n - 2 && x > xs[seg + 1]) seg++;
    if (x <= xs[0]) {
      lut[x] = clamp(Math.round(ys[0]), 0, 255);
      continue;
    }
    if (x >= xs[n - 1]) {
      lut[x] = clamp(Math.round(ys[n - 1]), 0, 255);
      continue;
    }

    const x0 = xs[seg];
    const x1 = xs[seg + 1];
    const y0 = ys[seg];
    const y1 = ys[seg + 1];
    const t = x1 !== x0 ? (x - x0) / (x1 - x0) : 0;
    const h = x1 - x0;

    const m0 = tangents[seg];
    const m1 = tangents[seg + 1];
    const t2 = t * t;
    const t3 = t2 * t;
    const h00 = 2 * t3 - 3 * t2 + 1;
    const h10 = t3 - 2 * t2 + t;
    const h01 = -2 * t3 + 3 * t2;
    const h11 = t3 - t2;
    const y = h00 * y0 + h10 * h * m0 + h01 * y1 + h11 * h * m1;
    lut[x] = clamp(Math.round(y), 0, 255);
  }

  return lut;
}

function clamp(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}

export function isIdentityCurve(points: CurvePoint[] | undefined): boolean {
  if (!points || points.length === 0) return true;
  const lut = buildCurveLut(points);
  for (let i = 0; i < 256; i++) {
    if (lut[i] !== i) return false;
  }
  return true;
}
