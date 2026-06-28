import { useCallback, useRef, useState } from "react";
import {
  aspectRatioValue,
  type AspectRatioPreset,
  type CropRect,
  type EditParams,
} from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { AdjustmentSlider } from "./AdjustmentSlider";
import { EditorSection } from "./EditorSection";

interface CropSectionProps {
  params: EditParams;
  imageWidth: number;
  imageHeight: number;
  onChange: (patch: Partial<EditParams>) => void;
  onCropChange: (crop: CropRect | undefined) => void;
}

const ASPECT_PRESETS: { key: AspectRatioPreset; labelKey: string }[] = [
  { key: "free", labelKey: "editor.aspectFree" },
  { key: "original", labelKey: "editor.aspectOriginal" },
  { key: "1:1", labelKey: "editor.aspectSquare" },
  { key: "16:9", labelKey: "16:9" },
  { key: "4:3", labelKey: "4:3" },
  { key: "3:2", labelKey: "3:2" },
  { key: "4:5", labelKey: "4:5" },
];

type Handle = "move" | "nw" | "ne" | "sw" | "se" | "n" | "s" | "e" | "w";

export function CropSection({
  params,
  imageWidth,
  imageHeight,
  onChange,
  onCropChange,
}: CropSectionProps) {
  const { t } = useTranslation();
  const originalRatio = imageWidth / Math.max(imageHeight, 1);
  const aspect = params.aspectRatio ?? "free";

  const applyAspect = useCallback(
    (preset: AspectRatioPreset) => {
      onChange({ aspectRatio: preset });
      const ratio = aspectRatioValue(preset, originalRatio);
      if (ratio == null) return;

      let width = 0.9;
      let height = width / ratio;
      if (height > 0.9) {
        height = 0.9;
        width = height * ratio;
      }
      onCropChange({
        x: (1 - width) / 2,
        y: (1 - height) / 2,
        width,
        height,
      });
    },
    [onCropChange, onChange, originalRatio],
  );

  return (
    <EditorSection title={t("editor.crop")}>
      <div className="flex flex-wrap gap-1.5">
        {ASPECT_PRESETS.map(({ key, labelKey }) => (
          <button
            key={key}
            type="button"
            onClick={() => applyAspect(key)}
            className={`rounded-md px-2.5 py-1 text-xs transition ${
              aspect === key
                ? "bg-blue-600 text-white"
                : "bg-white/10 text-neutral-300 hover:bg-white/15"
            }`}
          >
            {labelKey.startsWith("editor.") ? t(labelKey) : labelKey}
          </button>
        ))}
      </div>

      <AdjustmentSlider
        label={t("editor.straighten")}
        value={params.straighten}
        min={-45}
        max={45}
        onChange={(straighten) => onChange({ straighten })}
      />

      <div className="pt-1">
        <p className="mb-2 text-xs text-neutral-500">{t("editor.rotate")}</p>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => onChange({ rotate: (params.rotate - 90 + 360) % 360 })}
            className="flex-1 rounded-lg bg-white/10 py-2 text-xs text-neutral-200 transition hover:bg-white/15"
            title={t("editor.rotateLeft")}
          >
            ↺ {t("editor.rotateLeft")}
          </button>
          <button
            type="button"
            onClick={() => onChange({ rotate: (params.rotate + 90) % 360 })}
            className="flex-1 rounded-lg bg-white/10 py-2 text-xs text-neutral-200 transition hover:bg-white/15"
            title={t("editor.rotateRight")}
          >
            ↻ {t("editor.rotateRight")}
          </button>
        </div>
      </div>

      <div className="flex gap-2 pt-1">
        <button
          type="button"
          onClick={() => onChange({ flipH: !params.flipH })}
          className={`flex-1 rounded-lg py-2 text-xs transition ${
            params.flipH ? "bg-blue-600 text-white" : "bg-white/10 text-neutral-200 hover:bg-white/15"
          }`}
        >
          ↔ {t("editor.flipH")}
        </button>
        <button
          type="button"
          onClick={() => onChange({ flipV: !params.flipV })}
          className={`flex-1 rounded-lg py-2 text-xs transition ${
            params.flipV ? "bg-blue-600 text-white" : "bg-white/10 text-neutral-200 hover:bg-white/15"
          }`}
        >
          ↕ {t("editor.flipV")}
        </button>
      </div>
    </EditorSection>
  );
}

interface CropOverlayProps {
  crop: CropRect;
  aspectRatio?: AspectRatioPreset;
  originalRatio: number;
  onChange: (crop: CropRect) => void;
}

export function CropOverlay({ crop, aspectRatio, originalRatio, onChange }: CropOverlayProps) {
  const dragRef = useRef<{
    handle: Handle;
    startX: number;
    startY: number;
    startCrop: CropRect;
  } | null>(null);

  const ratio = aspectRatioValue(aspectRatio ?? "free", originalRatio);

  const startDrag = (handle: Handle, e: React.PointerEvent) => {
    e.stopPropagation();
    e.currentTarget.setPointerCapture(e.pointerId);
    dragRef.current = {
      handle,
      startX: e.clientX,
      startY: e.clientY,
      startCrop: { ...crop },
    };
  };

  const onPointerMove = (e: React.PointerEvent) => {
    const drag = dragRef.current;
    if (!drag) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const dx = (e.clientX - drag.startX) / rect.width;
    const dy = (e.clientY - drag.startY) / rect.height;
    let { x, y, width, height } = drag.startCrop;

    if (drag.handle === "move") {
      x = Math.max(0, Math.min(1 - width, x + dx));
      y = Math.max(0, Math.min(1 - height, y + dy));
    } else {
      if (drag.handle.includes("w")) {
        const nx = Math.max(0, Math.min(x + width - 0.05, x + dx));
        width = width + (x - nx);
        x = nx;
      }
      if (drag.handle.includes("e")) {
        width = Math.max(0.05, Math.min(1 - x, width + dx));
      }
      if (drag.handle.includes("n")) {
        const ny = Math.max(0, Math.min(y + height - 0.05, y + dy));
        height = height + (y - ny);
        y = ny;
      }
      if (drag.handle.includes("s")) {
        height = Math.max(0.05, Math.min(1 - y, height + dy));
      }
      if (ratio != null) {
        height = width / ratio;
        if (y + height > 1) {
          height = 1 - y;
          width = height * ratio;
        }
      }
    }

    onChange({ x, y, width, height });
  };

  const endDrag = () => {
    dragRef.current = null;
  };

  const handles: { id: Handle; className: string }[] = [
    { id: "nw", className: "left-0 top-0 -translate-x-1/2 -translate-y-1/2 cursor-nwse-resize" },
    { id: "ne", className: "right-0 top-0 translate-x-1/2 -translate-y-1/2 cursor-nesw-resize" },
    { id: "sw", className: "bottom-0 left-0 -translate-x-1/2 translate-y-1/2 cursor-nesw-resize" },
    { id: "se", className: "bottom-0 right-0 translate-x-1/2 translate-y-1/2 cursor-nwse-resize" },
    { id: "n", className: "left-1/2 top-0 -translate-x-1/2 -translate-y-1/2 cursor-ns-resize" },
    { id: "s", className: "bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2 cursor-ns-resize" },
    { id: "w", className: "left-0 top-1/2 -translate-x-1/2 -translate-y-1/2 cursor-ew-resize" },
    { id: "e", className: "right-0 top-1/2 translate-x-1/2 -translate-y-1/2 cursor-ew-resize" },
  ];

  return (
    <div
      className="absolute inset-0"
      onPointerMove={onPointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
    >
      <div
        className="absolute cursor-move border-2 border-white/90 shadow-[0_0_0_9999px_rgba(0,0,0,0.5)]"
        style={{
          left: `${crop.x * 100}%`,
          top: `${crop.y * 100}%`,
          width: `${crop.width * 100}%`,
          height: `${crop.height * 100}%`,
        }}
        onPointerDown={(e) => startDrag("move", e)}
      >
        <div className="pointer-events-none absolute inset-0 grid grid-cols-3 grid-rows-3">
          {Array.from({ length: 9 }).map((_, i) => (
            <div key={i} className="border border-white/20" />
          ))}
        </div>
        {handles.map(({ id, className }) => (
          <div
            key={id}
            className={`absolute h-3 w-3 rounded-full border-2 border-white bg-blue-500 ${className}`}
            onPointerDown={(e) => startDrag(id, e)}
          />
        ))}
      </div>
    </div>
  );
}

export function useCropMode(initialCrop?: CropRect) {
  const [cropActive, setCropActive] = useState(false);
  const [crop, setCrop] = useState<CropRect>(
    initialCrop ?? { x: 0.05, y: 0.05, width: 0.9, height: 0.9 },
  );
  return { cropActive, setCropActive, crop, setCrop };
}
