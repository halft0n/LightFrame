import { useCallback, useEffect, useMemo, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import {
  exportEdited,
  getEdit,
  getOriginalUrl,
  revertEdit,
  saveEdit,
} from "@/lib/tauri";
import {
  buildClipPath,
  buildCssFilter,
  buildImageTransform,
  DEFAULT_EDIT_PARAMS,
  isDefaultEditParams,
  parseEditParams,
  serializeEditParams,
  type CropRect,
  type EditParams,
} from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { CropOverlay, CropSection } from "./CropSection";
import { LightSection } from "./LightSection";
import { ColorSection } from "./ColorSection";
import { DetailSection } from "./DetailSection";
import { EffectsSection } from "./EffectsSection";

interface ImageEditorProps {
  mediaId: number;
  imagePath: string;
  filename: string;
  width: number;
  height: number;
  onClose: () => void;
  onSaved: () => void;
}

export function ImageEditor({
  mediaId,
  imagePath,
  filename,
  width,
  height,
  onClose,
  onSaved,
}: ImageEditorProps) {
  const { t } = useTranslation();
  const [params, setParams] = useState<EditParams>({ ...DEFAULT_EDIT_PARAMS });
  const [compare, setCompare] = useState(false);
  const [cropMode, setCropMode] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loaded, setLoaded] = useState(false);

  const imageSrc = getOriginalUrl(imagePath);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const saved = await getEdit(mediaId);
      if (cancelled) return;
      setParams(parseEditParams(saved));
    })();
    return () => {
      cancelled = true;
    };
  }, [mediaId]);

  const updateParams = useCallback((patch: Partial<EditParams>) => {
    setParams((prev) => ({ ...prev, ...patch }));
  }, []);

  const updateCrop = useCallback((crop: CropRect | undefined) => {
    setParams((prev) => ({ ...prev, crop }));
    setCropMode(crop != null);
  }, []);

  const handleReset = useCallback(() => {
    setParams({ ...DEFAULT_EDIT_PARAMS });
    setCropMode(false);
  }, []);

  const handleRevert = useCallback(async () => {
    await revertEdit(mediaId);
    setParams({ ...DEFAULT_EDIT_PARAMS });
    setCropMode(false);
    onSaved();
  }, [mediaId, onSaved]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const payload = isDefaultEditParams(params)
        ? null
        : serializeEditParams({ ...params, crop: cropMode ? params.crop : undefined });
      if (payload) {
        await saveEdit(mediaId, payload);
      } else {
        await revertEdit(mediaId);
      }
      onSaved();
      onClose();
    } finally {
      setSaving(false);
    }
  }, [cropMode, mediaId, onClose, onSaved, params]);

  const handleExport = useCallback(async () => {
    const defaultName = filename.replace(/\.[^.]+$/, "") + "_edited.jpg";
    const outputPath = await save({
      defaultPath: defaultName,
      filters: [{ name: "JPEG", extensions: ["jpg", "jpeg"] }],
    });
    if (!outputPath) return;

    const payload = serializeEditParams({ ...params, crop: cropMode ? params.crop : params.crop });
    await saveEdit(mediaId, payload);
    await exportEdited(mediaId, outputPath, 92);
  }, [cropMode, filename, mediaId, params]);

  const effectiveParams = compare ? DEFAULT_EDIT_PARAMS : params;
  const filter = useMemo(() => buildCssFilter(effectiveParams), [effectiveParams]);
  const transform = useMemo(() => buildImageTransform(effectiveParams), [effectiveParams]);
  const clipPath = useMemo(
    () => (compare ? undefined : buildClipPath(effectiveParams.crop)),
    [compare, effectiveParams.crop],
  );

  const vignetteStyle = useMemo(() => {
    if (compare || effectiveParams.vignette <= 0) return undefined;
    const amount = effectiveParams.vignette / 100;
    const radius = 40 + (effectiveParams.vignetteRadius / 100) * 40;
    return {
      background: `radial-gradient(ellipse at center, transparent ${radius}%, rgba(0,0,0,${amount * 0.7}) 100%)`,
    };
  }, [compare, effectiveParams.vignette, effectiveParams.vignetteRadius]);

  const crop = params.crop ?? { x: 0.05, y: 0.05, width: 0.9, height: 0.9 };
  const originalRatio = width / Math.max(height, 1);

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-[#0d0d0d] text-white">
      <header className="flex shrink-0 items-center justify-between border-b border-white/10 px-4 py-3">
        <h1 className="text-sm font-semibold tracking-wide text-neutral-200">{t("editor.title")}</h1>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onPointerDown={() => setCompare(true)}
            onPointerUp={() => setCompare(false)}
            onPointerLeave={() => setCompare(false)}
            className="rounded-lg px-3 py-1.5 text-xs text-neutral-300 transition hover:bg-white/10 active:bg-white/20"
          >
            {t("editor.compare")}
          </button>
          <button
            type="button"
            onClick={handleReset}
            className="rounded-lg px-3 py-1.5 text-xs text-neutral-300 transition hover:bg-white/10"
          >
            {t("editor.reset")}
          </button>
          <button
            type="button"
            onClick={handleRevert}
            className="rounded-lg px-3 py-1.5 text-xs text-neutral-300 transition hover:bg-white/10"
          >
            {t("editor.revert")}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg px-3 py-1.5 text-xs text-neutral-300 transition hover:bg-white/10"
          >
            {t("editor.cancel")}
          </button>
          <button
            type="button"
            onClick={() => void handleExport()}
            className="rounded-lg px-3 py-1.5 text-xs text-neutral-300 transition hover:bg-white/10"
          >
            {t("editor.export")}
          </button>
          <button
            type="button"
            onClick={() => void handleSave()}
            disabled={saving}
            className="rounded-lg bg-blue-600 px-4 py-1.5 text-xs font-medium text-white transition hover:bg-blue-500 disabled:opacity-50"
          >
            {t("editor.save")}
          </button>
        </div>
      </header>

      <div className="flex min-h-0 flex-1">
        <div className="relative flex flex-1 items-center justify-center overflow-hidden bg-[#080808] p-6">
          <div className="relative inline-block max-h-full max-w-full">
            <img
              src={imageSrc}
              alt={filename}
              onLoad={() => setLoaded(true)}
              draggable={false}
              className="max-h-[calc(100vh-8rem)] max-w-full select-none object-contain transition-[filter,transform,clip-path] duration-150 ease-out"
              style={{
                filter,
                transform,
                clipPath,
              }}
            />
            {loaded && cropMode && !compare && (
              <CropOverlay
                crop={crop}
                aspectRatio={params.aspectRatio}
                originalRatio={originalRatio}
                onChange={(next) => updateCrop(next)}
              />
            )}
            {vignetteStyle && (
              <div
                className="pointer-events-none absolute inset-0 transition-opacity duration-150"
                style={vignetteStyle}
              />
            )}
          </div>
        </div>

        <aside className="flex w-80 shrink-0 flex-col border-l border-white/10 bg-[#141414]">
          <div className="flex-1 overflow-y-auto">
            <CropSection
              params={params}
              imageWidth={width}
              imageHeight={height}
              onChange={(patch) => {
                if (patch.aspectRatio != null) setCropMode(true);
                updateParams(patch);
              }}
              onCropChange={updateCrop}
            />
            <LightSection params={params} onChange={updateParams} />
            <ColorSection params={params} onChange={updateParams} />
            <DetailSection params={params} onChange={updateParams} />
            <EffectsSection params={params} onChange={updateParams} />
          </div>
        </aside>
      </div>
    </div>
  );
}
