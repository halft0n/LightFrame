import { useCallback, useEffect, useRef, useState } from "react";
import {
  getMediaById,
  getMediaList,
  getMediaNeighbors,
  getOriginalUrl,
  getThumbnailUrl,
  getEdit,
  hasEdits,
  toggleFavorite,
  type MediaItem,
} from "@/lib/tauri";
import { buildClipPath, buildCssFilter, buildImageTransform, parseEditParams } from "@/lib/editParams";
import { closeViewer, openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";
import { VideoPlayer } from "./VideoPlayer";
import { ImageEditor } from "@/components/editor/ImageEditor";

interface PhotoViewerProps {
  mediaId: number;
}

const MIN_ZOOM = 1;
const MAX_ZOOM = 5;
const FILMSTRIP_SIZE = 20;

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
  return `${(bytes / 1073741824).toFixed(1)} GB`;
}

function formatMediaDate(item: MediaItem, locale: string): string {
  const raw = item.created_at ?? item.modified_at;
  if (!raw) return "—";
  const date = new Date(raw);
  return new Intl.DateTimeFormat(locale === "zh-CN" ? "zh-CN" : "en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function PhotoViewer({ mediaId }: PhotoViewerProps) {
  const { t, locale } = useTranslation();
  const [media, setMedia] = useState<MediaItem | null>(null);
  const [neighbors, setNeighbors] = useState<{ prev_id: number | null; next_id: number | null }>({
    prev_id: null,
    next_id: null,
  });
  const [filmstrip, setFilmstrip] = useState<MediaItem[]>([]);
  const [showInfo, setShowInfo] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [useOriginal, setUseOriginal] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);
  const [isFavorite, setIsFavorite] = useState(false);
  const [edited, setEdited] = useState(false);
  const [editParamsJson, setEditParamsJson] = useState<string | null>(null);
  const [previewKey, setPreviewKey] = useState(0);
  const dragStart = useRef({ x: 0, y: 0, panX: 0, panY: 0 });
  const filmstripRef = useRef<HTMLDivElement>(null);

  const resetView = useCallback(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
    setUseOriginal(false);
  }, []);

  useEffect(() => {
    resetView();
    setIsFavorite(false);
    let cancelled = false;

    void (async () => {
      const [item, nb, list, hasEdit] = await Promise.all([
        getMediaById(mediaId),
        getMediaNeighbors(mediaId),
        getMediaList(0, FILMSTRIP_SIZE * 3),
        hasEdits(mediaId),
      ]);
      if (cancelled) return;
      setMedia(item);
      setNeighbors(nb);
      setFilmstrip(list);
      setEdited(hasEdit);
      if (hasEdit) {
        setEditParamsJson(await getEdit(mediaId));
      } else {
        setEditParamsJson(null);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [mediaId, resetView]);

  useEffect(() => {
    const el = filmstripRef.current;
    if (!el) return;
    const active = el.querySelector(`[data-id="${mediaId}"]`);
    active?.scrollIntoView({ inline: "center", block: "nearest", behavior: "smooth" });
  }, [mediaId, filmstrip]);

  const navigate = useCallback((id: number | null) => {
    if (id != null) openViewer(id);
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        closeViewer();
        return;
      }
      if (media?.media_type === "Video") return;
      if (e.key === "ArrowLeft") {
        navigate(neighbors.prev_id);
      } else if (e.key === "ArrowRight") {
        navigate(neighbors.next_id);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [navigate, neighbors, media?.media_type]);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      e.preventDefault();
      const delta = e.deltaY > 0 ? -0.15 : 0.15;
      setZoom((z) => {
        const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z + delta));
        if (next <= 1) setPan({ x: 0, y: 0 });
        return next;
      });
    },
    [],
  );

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (zoom <= 1) return;
      setDragging(true);
      dragStart.current = { x: e.clientX, y: e.clientY, panX: pan.x, panY: pan.y };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [pan, zoom],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging) return;
      setPan({
        x: dragStart.current.panX + (e.clientX - dragStart.current.x),
        y: dragStart.current.panY + (e.clientY - dragStart.current.y),
      });
    },
    [dragging],
  );

  const handlePointerUp = useCallback(() => {
    setDragging(false);
  }, []);

  const handleLargeLoaded = useCallback(() => {
    setUseOriginal(true);
  }, []);

  const handleShare = useCallback(async () => {
    if (!media || !navigator.share) return;
    try {
      await navigator.share({ title: media.filename, text: media.filename });
    } catch {
      // User cancelled or share unavailable
    }
  }, [media]);

  const handleToggleFavorite = useCallback(async () => {
    const next = await toggleFavorite(mediaId);
    setIsFavorite(next);
  }, [mediaId]);

  const handleZoomChange = useCallback((value: number) => {
    setZoom(value);
    if (value <= 1) setPan({ x: 0, y: 0 });
  }, []);

  const handleEditorSaved = useCallback(async () => {
    const hasEdit = await hasEdits(mediaId);
    setEdited(hasEdit);
    if (hasEdit) {
      setEditParamsJson(await getEdit(mediaId));
    } else {
      setEditParamsJson(null);
    }
    setPreviewKey((k) => k + 1);
    resetView();
  }, [mediaId, resetView]);

  const editPreview = editParamsJson ? parseEditParams(editParamsJson) : null;
  const previewFilter = editPreview ? buildCssFilter(editPreview) : undefined;
  const previewTransform = editPreview ? buildImageTransform(editPreview) : undefined;
  const previewClip = editPreview ? buildClipPath(editPreview.crop) : undefined;

  const isVideo = media?.media_type === "Video";
  const imageSrc = media
    ? useOriginal
      ? getOriginalUrl(media.path)
      : getThumbnailUrl(media.id, "large")
    : "";

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden bg-neutral-950 text-white">
      <div className="flex shrink-0 items-center gap-3 border-b border-white/10 px-4 py-2">
        <button
          type="button"
          onClick={closeViewer}
          className="rounded-lg px-2 py-1.5 text-lg leading-none text-neutral-300 transition hover:bg-white/10"
          title={t("viewer.back")}
          aria-label={t("viewer.back")}
        >
          ‹
        </button>

        {!isVideo && (
          <input
            type="range"
            min={MIN_ZOOM}
            max={MAX_ZOOM}
            step={0.05}
            value={zoom}
            onChange={(e) => handleZoomChange(Number(e.target.value))}
            className="h-1 w-24 shrink-0 cursor-pointer accent-blue-500"
            aria-label={t("viewer.zoom")}
          />
        )}

        <p className="min-w-0 flex-1 truncate text-center text-sm font-medium text-neutral-100">
          {media ? formatMediaDate(media, locale) : "—"}
        </p>

        <div className="flex shrink-0 items-center gap-1">
          {edited && (
            <span className="rounded-full bg-blue-600/80 px-2 py-0.5 text-xs font-medium">
              {t("editor.hasEdits")}
            </span>
          )}
          <button
            type="button"
            onClick={() => setShowInfo((v) => !v)}
            className={`rounded-lg px-3 py-1.5 text-sm transition hover:bg-white/10 ${
              showInfo ? "bg-white/10 text-white" : "text-neutral-300"
            }`}
            title={t("viewer.info")}
          >
            {t("viewer.info")}
          </button>
          <button
            type="button"
            onClick={() => void handleShare()}
            disabled={!media || typeof navigator.share !== "function"}
            className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10 disabled:opacity-30"
            title={t("viewer.share")}
            aria-label={t("viewer.share")}
          >
            ↗
          </button>
          <button
            type="button"
            onClick={() => void handleToggleFavorite()}
            className={`rounded-lg px-3 py-1.5 text-sm transition hover:bg-white/10 ${
              isFavorite ? "text-red-400" : "text-neutral-300"
            }`}
            title={t("viewer.favorite")}
            aria-label={t("viewer.favorite")}
          >
            {isFavorite ? "♥" : "♡"}
          </button>
          {!isVideo && (
            <button
              type="button"
              onClick={() => setEditorOpen(true)}
              className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10"
            >
              {t("viewer.edit")}
            </button>
          )}
        </div>
      </div>

      <div className="relative flex flex-1 overflow-hidden">
        {isVideo && media ? (
          <VideoPlayer
            src={getOriginalUrl(media.path)}
            mediaId={mediaId}
            filmstripIds={filmstrip.map((item) => item.id)}
            onNavigate={openViewer}
          />
        ) : (
          <>
            <div
              className={`flex flex-1 items-center justify-center overflow-hidden ${
                zoom > 1 ? (dragging ? "cursor-grabbing" : "cursor-grab") : ""
              }`}
              onWheel={handleWheel}
              onPointerDown={handlePointerDown}
              onPointerMove={handlePointerMove}
              onPointerUp={handlePointerUp}
              onPointerCancel={handlePointerUp}
            >
              {media && (
                <>
                  {!useOriginal && (
                    <img
                      src={getThumbnailUrl(media.id, "large")}
                      alt={media.filename}
                      onLoad={handleLargeLoaded}
                      className="hidden"
                      aria-hidden="true"
                    />
                  )}
                  <div
                    style={{
                      transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
                      transition: dragging ? "none" : "transform 0.1s ease-out",
                    }}
                  >
                    <img
                      key={previewKey}
                      src={imageSrc}
                      alt={media.filename}
                      draggable={false}
                      style={{
                        transform: previewTransform,
                        maxHeight: "100%",
                        maxWidth: "100%",
                        objectFit: "contain",
                        filter: previewFilter,
                        clipPath: previewClip,
                      }}
                      className="select-none"
                    />
                  </div>
                </>
              )}
            </div>

            {showInfo && media && (
              <aside className="w-72 shrink-0 overflow-y-auto border-l border-white/10 bg-black/60 p-4 text-sm">
                <dl className="space-y-3">
                  <div>
                    <dt className="text-neutral-500">{t("viewer.filename")}</dt>
                    <dd className="mt-0.5 break-all text-neutral-200">{media.filename}</dd>
                  </div>
                  <div>
                    <dt className="text-neutral-500">{t("viewer.size")}</dt>
                    <dd className="mt-0.5 text-neutral-200">{formatFileSize(media.size_bytes)}</dd>
                  </div>
                  <div>
                    <dt className="text-neutral-500">{t("viewer.date")}</dt>
                    <dd className="mt-0.5 text-neutral-200">{formatMediaDate(media, locale)}</dd>
                  </div>
                  {media.width != null && media.height != null && (
                    <div>
                      <dt className="text-neutral-500">{t("viewer.dimensions")}</dt>
                      <dd className="mt-0.5 text-neutral-200">
                        {media.width} × {media.height}
                      </dd>
                    </div>
                  )}
                  <div>
                    <dt className="text-neutral-500">{t("viewer.type")}</dt>
                    <dd className="mt-0.5 text-neutral-200">{media.media_type}</dd>
                  </div>
                  {(media.latitude != null || media.longitude != null) && (
                    <div>
                      <dt className="text-neutral-500">{t("viewer.location")}</dt>
                      <dd className="mt-0.5 text-neutral-200">
                        {media.latitude?.toFixed(4)}, {media.longitude?.toFixed(4)}
                      </dd>
                    </div>
                  )}
                </dl>
              </aside>
            )}
          </>
        )}
      </div>

      {!isVideo && (
        <div
          ref={filmstripRef}
          className="flex shrink-0 gap-2 overflow-x-auto border-t border-white/10 px-4 py-3"
        >
          {filmstrip.map((item) => (
            <button
              key={item.id}
              type="button"
              data-id={item.id}
              onClick={() => openViewer(item.id)}
              className={`h-16 w-16 shrink-0 overflow-hidden rounded-md transition ${
                item.id === mediaId ? "ring-2 ring-blue-500" : "opacity-70 hover:opacity-100"
              }`}
            >
              <img
                src={getThumbnailUrl(item.id, "small")}
                alt={item.filename}
                className="h-full w-full object-cover"
              />
            </button>
          ))}
        </div>
      )}

      <div className="shrink-0 border-t border-white/10 px-4 py-1.5 text-center text-xs text-neutral-400">
        <span className="truncate">{media?.filename ?? "—"}</span>
      </div>

      {editorOpen && media && (
        <ImageEditor
          mediaId={mediaId}
          imagePath={media.path}
          filename={media.filename}
          width={media.width ?? 1920}
          height={media.height ?? 1080}
          onClose={() => setEditorOpen(false)}
          onSaved={() => void handleEditorSaved()}
        />
      )}
    </div>
  );
}
