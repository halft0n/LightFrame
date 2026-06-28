import { useCallback, useEffect, useRef, useState } from "react";
import {
  DEFAULT_CURVE_POINTS,
  type CurvePoint,
  buildCurveLut,
  isIdentityCurve,
  sortCurvePoints,
} from "@/lib/curves";
import type { EditParams } from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { EditorSection } from "./EditorSection";

type ChannelKey = "rgb" | "r" | "g" | "b";

const CANVAS_SIZE = 256;
const POINT_RADIUS = 5;

interface CurvesSectionProps {
  params: EditParams;
  onChange: (patch: Partial<EditParams>) => void;
}

function getChannelPoints(curves: EditParams["curves"], channel: ChannelKey): CurvePoint[] {
  if (!curves) return [...DEFAULT_CURVE_POINTS];
  const pts = channel === "rgb" ? curves.rgb : curves[channel];
  return pts?.length ? [...pts] : [...DEFAULT_CURVE_POINTS];
}

function clamp(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}

export function CurvesSection({ params, onChange }: CurvesSectionProps) {
  const { t } = useTranslation();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [channel, setChannel] = useState<ChannelKey>("rgb");
  const dragRef = useRef<{ index: number } | null>(null);

  const curves = params.curves ?? { rgb: [...DEFAULT_CURVE_POINTS] };
  const points = getChannelPoints(curves, channel);

  const updateChannelPoints = useCallback(
    (nextPoints: CurvePoint[]) => {
      const sorted = sortCurvePoints(nextPoints);
      const nextCurves = { ...curves, [channel]: sorted };
      if (channel === "rgb") nextCurves.rgb = sorted;
      onChange({ curves: nextCurves });
    },
    [channel, curves, onChange],
  );

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, CANVAS_SIZE, CANVAS_SIZE);

    ctx.fillStyle = "#1a1a1a";
    ctx.fillRect(0, 0, CANVAS_SIZE, CANVAS_SIZE);

    ctx.strokeStyle = "#333";
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
      const pos = (i / 4) * CANVAS_SIZE;
      ctx.beginPath();
      ctx.moveTo(pos, 0);
      ctx.lineTo(pos, CANVAS_SIZE);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(0, pos);
      ctx.lineTo(CANVAS_SIZE, pos);
      ctx.stroke();
    }

    ctx.strokeStyle = "#555";
    ctx.beginPath();
    ctx.moveTo(0, CANVAS_SIZE);
    ctx.lineTo(CANVAS_SIZE, 0);
    ctx.stroke();

    const lut = buildCurveLut(points);
    ctx.strokeStyle =
      channel === "rgb"
        ? "#ccc"
        : channel === "r"
          ? "#ef4444"
          : channel === "g"
            ? "#22c55e"
            : "#3b82f6";
    ctx.lineWidth = 2;
    ctx.beginPath();
    for (let x = 0; x < 256; x++) {
      const y = 255 - lut[x];
      if (x === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();

    const sorted = sortCurvePoints(points);
    for (let i = 0; i < sorted.length; i++) {
      const [px, py] = sorted[i];
      const cx = px;
      const cy = CANVAS_SIZE - 1 - py;
      ctx.beginPath();
      ctx.arc(cx, cy, POINT_RADIUS, 0, Math.PI * 2);
      ctx.fillStyle = dragRef.current?.index === i ? "#fff" : "#3b82f6";
      ctx.fill();
      ctx.strokeStyle = "#fff";
      ctx.lineWidth = 1.5;
      ctx.stroke();
    }
  }, [channel, points]);

  useEffect(() => {
    draw();
  }, [draw]);

  const canvasToPoint = (clientX: number, clientY: number): CurvePoint | null => {
    const canvas = canvasRef.current;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    const scaleX = CANVAS_SIZE / rect.width;
    const scaleY = CANVAS_SIZE / rect.height;
    const x = clamp(Math.round((clientX - rect.left) * scaleX), 0, 255);
    const y = clamp(Math.round(255 - (clientY - rect.top) * scaleY), 0, 255);
    return [x, y];
  };

  const findPointIndex = (x: number, y: number): number => {
    const sorted = sortCurvePoints(points);
    for (let i = 0; i < sorted.length; i++) {
      const [px, py] = sorted[i];
      const dx = px - x;
      const dy = py - y;
      if (Math.sqrt(dx * dx + dy * dy) <= POINT_RADIUS + 4) return i;
    }
    return -1;
  };

  const handlePointerDown = (e: React.PointerEvent) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    const pt = canvasToPoint(e.clientX, e.clientY);
    if (!pt) return;
    const sorted = sortCurvePoints(points);
    const idx = findPointIndex(pt[0], pt[1]);
    if (idx >= 0) {
      dragRef.current = { index: idx };
    } else {
      const next = sortCurvePoints([...sorted, pt]);
      dragRef.current = { index: next.findIndex((p) => p[0] === pt[0] && p[1] === pt[1]) };
      updateChannelPoints(next);
    }
    draw();
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!dragRef.current) return;
    const pt = canvasToPoint(e.clientX, e.clientY);
    if (!pt) return;
    const sorted = sortCurvePoints(points);
    const idx = dragRef.current.index;
    let [x, y] = pt;

    if (idx === 0) x = 0;
    else if (idx === sorted.length - 1) x = 255;
    else x = Math.max(1, Math.min(254, x));

    const next = sorted.map((p, i) => (i === idx ? ([x, y] as CurvePoint) : p));
    updateChannelPoints(next);
  };

  const handlePointerUp = () => {
    dragRef.current = null;
    draw();
  };

  const handleReset = () => {
    updateChannelPoints([...DEFAULT_CURVE_POINTS]);
  };

  const channels: { key: ChannelKey; label: string }[] = [
    { key: "rgb", label: t("editor.channel.rgb") },
    { key: "r", label: t("editor.channel.r") },
    { key: "g", label: t("editor.channel.g") },
    { key: "b", label: t("editor.channel.b") },
  ];

  const channelPoints = channel === "rgb" ? curves.rgb : curves[channel];
  const isDefault = isIdentityCurve(channelPoints);

  return (
    <EditorSection title={t("editor.curves")} defaultOpen={false}>
      <div className="flex gap-1">
        {channels.map(({ key, label }) => (
          <button
            key={key}
            type="button"
            onClick={() => setChannel(key)}
            className={`flex-1 rounded-md px-2 py-1 text-xs transition ${
              channel === key
                ? "bg-blue-600 text-white"
                : "bg-white/10 text-neutral-300 hover:bg-white/15"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="relative mx-auto w-full max-w-[256px]">
        <canvas
          ref={canvasRef}
          width={CANVAS_SIZE}
          height={CANVAS_SIZE}
          className="w-full cursor-crosshair rounded-lg border border-white/10 touch-none"
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerCancel={handlePointerUp}
        />
      </div>

      <div className="flex items-center justify-between pt-1">
        <span className="text-xs text-neutral-500">
          {points.length} {t("editor.curves")} pts
        </span>
        <button
          type="button"
          onClick={handleReset}
          disabled={isDefault}
          className="rounded-md bg-white/10 px-2.5 py-1 text-xs text-neutral-300 transition hover:bg-white/15 disabled:opacity-40"
        >
          {t("editor.reset")}
        </button>
      </div>
    </EditorSection>
  );
}
