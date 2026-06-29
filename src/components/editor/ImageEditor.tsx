import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import { CurvesSection } from "./CurvesSection";
import { LevelsSection } from "./LevelsSection";
import { SelectiveColorSection } from "./SelectiveColorSection";
import { DetailSection } from "./DetailSection";
import { EffectsSection } from "./EffectsSection";

const MAX_HISTORY = 50;

function cloneEditParams(params: EditParams): EditParams {
  return JSON.parse(JSON.stringify(params)) as EditParams;
}

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
  const [saveError, setSaveError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [historyMeta, setHistoryMeta] = useState({ index: 0, total: 1 });

  const historyRef = useRef<EditParams[]>([cloneEditParams(DEFAULT_EDIT_PARAMS)]);
  const historyIndexRef = useRef(0);
  const skipHistoryRef = useRef(false);

  const imageSrc = getOriginalUrl(imagePath);

  const initHistory = useCallback((initial: EditParams) => {
    historyRef.current = [cloneEditParams(initial)];
    historyIndexRef.current = 0;
    setHistoryMeta({ index: 0, total: 1 });
  }, []);

  const historyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingSnapshotRef = useRef<EditParams | null>(null);

  const commitHistory = useCallback((snapshot: EditParams) => {
    const idx = historyIndexRef.current;
    let stack = historyRef.current.slice(0, idx + 1);
    stack.push(cloneEditParams(snapshot));
    if (stack.length > MAX_HISTORY) {
      stack = stack.slice(stack.length - MAX_HISTORY);
    }
    historyRef.current = stack;
    historyIndexRef.current = stack.length - 1;
    setHistoryMeta({ index: historyIndexRef.current, total: stack.length });
  }, []);

  const pushHistory = useCallback((next: EditParams) => {
    pendingSnapshotRef.current = next;
    if (historyTimerRef.current) {
      clearTimeout(historyTimerRef.current);
    }
    historyTimerRef.current = setTimeout(() => {
      if (pendingSnapshotRef.current) {
        commitHistory(pendingSnapshotRef.current);
        pendingSnapshotRef.current = null;
      }
      historyTimerRef.current = null;
    }, 300);
  }, [commitHistory]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const saved = await getEdit(mediaId);
      if (cancelled) return;
      const loadedParams = parseEditParams(saved);
      skipHistoryRef.current = true;
      setParams(loadedParams);
      initHistory(loadedParams);
      skipHistoryRef.current = false;
    })();
    return () => {
      cancelled = true;
      if (historyTimerRef.current) {
        clearTimeout(historyTimerRef.current);
        historyTimerRef.current = null;
      }
    };
  }, [mediaId, initHistory]);

  const applyParams = useCallback(
    (next: EditParams) => {
      setParams(next);
      if (!skipHistoryRef.current) {
        pushHistory(next);
      }
    },
    [pushHistory],
  );

  const updateParams = useCallback(
    (patch: Partial<EditParams>) => {
      setParams((prev) => {
        const next = { ...prev, ...patch };
        if (!skipHistoryRef.current) {
          pushHistory(next);
        }
        return next;
      });
    },
    [pushHistory],
  );

  const updateCrop = useCallback(
    (crop: CropRect | undefined) => {
      setParams((prev) => {
        const next = { ...prev, crop };
        if (!skipHistoryRef.current) {
          pushHistory(next);
        }
        return next;
      });
      setCropMode(crop != null);
    },
    [pushHistory],
  );

  const canUndo = historyMeta.index > 0;
  const canRedo = historyMeta.index < historyMeta.total - 1;

  const undo = useCallback(() => {
    if (historyIndexRef.current <= 0) return;
    historyIndexRef.current -= 1;
    const snapshot = cloneEditParams(historyRef.current[historyIndexRef.current]);
    skipHistoryRef.current = true;
    setParams(snapshot);
    skipHistoryRef.current = false;
    setHistoryMeta({ index: historyIndexRef.current, total: historyRef.current.length });
  }, []);

  const redo = useCallback(() => {
    if (historyIndexRef.current >= historyRef.current.length - 1) return;
    historyIndexRef.current += 1;
    const snapshot = cloneEditParams(historyRef.current[historyIndexRef.current]);
    skipHistoryRef.current = true;
    setParams(snapshot);
    skipHistoryRef.current = false;
    setHistoryMeta({ index: historyIndexRef.current, total: historyRef.current.length });
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.key.toLowerCase() !== "z") return;
      e.preventDefault();
      if (e.shiftKey) redo();
      else undo();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [redo, undo]);

  const handleReset = useCallback(() => {
    const next = { ...DEFAULT_EDIT_PARAMS };
    applyParams(next);
    setCropMode(false);
  }, [applyParams]);

  const handleRevert = useCallback(async () => {
    await revertEdit(mediaId);
    const next = { ...DEFAULT_EDIT_PARAMS };
    skipHistoryRef.current = true;
    setParams(next);
    initHistory(next);
    skipHistoryRef.current = false;
    setCropMode(false);
    onSaved();
  }, [initHistory, mediaId, onSaved]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveError(null);
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
    } catch (err) {
      console.error("Failed to save edits:", err);
      setSaveError(t("editor.saveError"));
    } finally {
      setSaving(false);
    }
  }, [cropMode, mediaId, onClose, onSaved, params, t]);

  const handleExport = useCallback(async () => {
    const defaultName = filename.replace(/\.[^.]+$/, "") + "_edited.jpg";
    const outputPath = await save({
      defaultPath: defaultName,
      filters: [{ name: "JPEG", extensions: ["jpg", "jpeg"] }],
    });
    if (!outputPath) return;

    const exportParams = { ...params };
    if (!cropMode) {
      exportParams.crop = undefined;
    }
    const payload = serializeEditParams(exportParams);
    const previousEdit = await getEdit(mediaId);
    try {
      await saveEdit(mediaId, payload);
      await exportEdited(mediaId, outputPath, 92);
    } catch (err) {
      console.error("Failed to export edited image:", err);
      setSaveError(t("editor.exportError"));
    } finally {
      if (previousEdit) {
        await saveEdit(mediaId, previousEdit);
      } else {
        await revertEdit(mediaId);
      }
    }
  }, [cropMode, filename, mediaId, params, t]);

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
          {saveError && (
            <span className="text-xs text-red-400">{saveError}</span>
          )}
          <div className="flex items-center gap-1 rounded-lg bg-white/5 px-1 py-0.5">
            <button
              type="button"
              onClick={undo}
              disabled={!canUndo}
              title={`${t("editor.undo")} (Ctrl+Z)`}
              className="rounded-md px-2 py-1 text-xs text-neutral-300 transition hover:bg-white/10 disabled:opacity-30"
            >
              ↶ {t("editor.undo")}
            </button>
            <button
              type="button"
              onClick={redo}
              disabled={!canRedo}
              title={`${t("editor.redo")} (Ctrl+Shift+Z)`}
              className="rounded-md px-2 py-1 text-xs text-neutral-300 transition hover:bg-white/10 disabled:opacity-30"
            >
              ↷ {t("editor.redo")}
            </button>
            <span className="px-1.5 text-[10px] tabular-nums text-neutral-500">
              {historyMeta.index + 1}/{historyMeta.total}
            </span>
          </div>
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
            onClick={() => void handleRevert()}
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
                transformStyle: hasPerspective(params) ? "preserve-3d" : undefined,
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
            <CurvesSection params={params} onChange={updateParams} />
            <LevelsSection params={params} onChange={updateParams} />
            <SelectiveColorSection params={params} onChange={updateParams} />
            <DetailSection params={params} onChange={updateParams} />
            <EffectsSection params={params} onChange={updateParams} />
          </div>
        </aside>
      </div>
    </div>
  );
}

function hasPerspective(params: EditParams): boolean {
  return params.perspectiveV !== 0 || params.perspectiveH !== 0;
}
