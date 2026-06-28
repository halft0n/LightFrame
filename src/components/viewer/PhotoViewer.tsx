import { useCallback, useEffect, useRef, useState } from "react";
import {
  getMediaById,
  getMediaNeighbors,
  getOriginalUrl,
  getThumbnailUrl,
  getEdit,
  hasEdits,
  toggleFavorite,
  getFavoriteState,
  deleteMedia,
  type MediaItem,
} from "@/lib/tauri";
import { buildClipPath, buildCssFilter, buildImageTransform, parseEditParams } from "@/lib/editParams";
import { isTypingTarget } from "@/lib/keyboard";
import { closeViewer, loadMedia, openViewer, useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";
import { VideoPlayer } from "./VideoPlayer";
import { ImageEditor } from "@/components/editor/ImageEditor";
import { InfoPanel } from "./InfoPanel";
import { SimilarPhotosPanel } from "./SimilarPhotosPanel";
import { useImagePreloader } from "@/hooks/useImagePreloader";

interface PhotoViewerProps {
  mediaId: number;
}

const MIN_ZOOM = 1;
const MAX_ZOOM = 5;
const FILMSTRIP_SIZE = 20;
const VIEWER_EXIT_MS = 200;

function formatMediaDate(item: MediaItem, locale: string, notAvailable: string): string {
  const raw = item.created_at ?? item.modified_at;
  if (!raw) return notAvailable;
  const date = new Date(raw);
  return new Intl.DateTimeFormat(locale === "zh-CN" ? "zh-CN" : "en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function filmstripFromContext(
  mediaId: number,
  contextItems: MediaItem[],
  radius: number,
): MediaItem[] | null {
  const idx = contextItems.findIndex((m) => m.id === mediaId);
  if (idx === -1) return null;
  const start = Math.max(0, idx - radius);
  const end = Math.min(contextItems.length, idx + radius + 1);
  return contextItems.slice(start, end);
}

async function loadFilmstripAroundMedia(mediaId: number, radius: number): Promise<MediaItem[]> {
  const prevIds: number[] = [];
  const visited = new Set<number>([mediaId]);
  let cursor = mediaId;
  for (let i = 0; i < radius; i++) {
    const nb = await getMediaNeighbors(cursor);
    if (nb.prev_id == null || visited.has(nb.prev_id)) break;
    prevIds.unshift(nb.prev_id);
    visited.add(nb.prev_id);
    cursor = nb.prev_id;
  }

  const nextIds: number[] = [];
  cursor = mediaId;
  for (let i = 0; i < radius; i++) {
    const nb = await getMediaNeighbors(cursor);
    if (nb.next_id == null || visited.has(nb.next_id)) break;
    nextIds.push(nb.next_id);
    visited.add(nb.next_id);
    cursor = nb.next_id;
  }

  const ids = [...prevIds, mediaId, ...nextIds];
  const items = await Promise.all(ids.map((id) => getMediaById(id)));
  return items.filter((item): item is MediaItem => item != null);
}

async function loadContextFilmstrip(
  mediaId: number,
  contextItems: MediaItem[],
  radius: number,
): Promise<MediaItem[]> {
  const fromContext = filmstripFromContext(mediaId, contextItems, radius);
  if (fromContext && fromContext.length > 0) return fromContext;
  return loadFilmstripAroundMedia(mediaId, radius);
}

export function PhotoViewer({ mediaId }: PhotoViewerProps) {
  const { t, locale } = useTranslation();
  const { mediaItems: contextItems } = useAppStore();
  const [media, setMedia] = useState<MediaItem | null>(null);
  const [neighbors, setNeighbors] = useState<{ prev_id: number | null; next_id: number | null }>({
    prev_id: null,
    next_id: null,
  });
  const [filmstrip, setFilmstrip] = useState<MediaItem[]>([]);
  const [showInfo, setShowInfo] = useState(false);
  const [showSimilar, setShowSimilar] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [useOriginal, setUseOriginal] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);
  const [isFavorite, setIsFavorite] = useState(false);
  const [edited, setEdited] = useState(false);
  const [editParamsJson, setEditParamsJson] = useState<string | null>(null);
  const [previewKey, setPreviewKey] = useState(0);
  const [rotation, setRotation] = useState(0);
  const [closing, setClosing] = useState(false);
  const [currentImageLoaded, setCurrentImageLoaded] = useState(false);
  const dragStart = useRef({ x: 0, y: 0, panX: 0, panY: 0 });
  const filmstripRef = useRef<HTMLDivElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);

  const requestClose = useCallback(() => {
    if (closing) return;
    setClosing(true);
  }, [closing]);

  useEffect(() => {
    if (!closing) return;
    const timer = window.setTimeout(() => closeViewer(), VIEWER_EXIT_MS);
    return () => window.clearTimeout(timer);
  }, [closing]);

  const resetView = useCallback(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
    setUseOriginal(false);
    setRotation(0);
  }, []);

  useEffect(() => {
    setCurrentImageLoaded(false);
  }, [mediaId, previewKey, useOriginal]);

  useImagePreloader({
    mediaId,
    filmstrip,
    currentImageLoaded,
    enabled: media?.media_type !== "Video",
  });

  useEffect(() => {
    resetView();
    setShowSimilar(false);
    let cancelled = false;

    void (async () => {
      const [item, nb, strip, hasEdit, favoriteState] = await Promise.all([
        getMediaById(mediaId),
        getMediaNeighbors(mediaId),
        loadContextFilmstrip(mediaId, contextItems, FILMSTRIP_SIZE),
        hasEdits(mediaId),
        getFavoriteState(mediaId),
      ]);
      if (cancelled) return;
      setMedia(item);
      setNeighbors(nb);
      setFilmstrip(strip);
      setEdited(hasEdit);
      setIsFavorite(favoriteState);
      if (hasEdit) {
        setEditParamsJson(await getEdit(mediaId));
      } else {
        setEditParamsJson(null);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [mediaId, resetView, contextItems]);

  useEffect(() => {
    dialogRef.current?.focus();
  }, []);

  useEffect(() => {
    const el = filmstripRef.current;
    if (!el) return;
    const active = el.querySelector(`[data-id="${mediaId}"]`);
    active?.scrollIntoView({ inline: "center", block: "nearest", behavior: "smooth" });
  }, [mediaId, filmstrip]);

  const navigate = useCallback((id: number | null) => {
    if (id != null) openViewer(id);
  }, []);

  const handleToggleFavorite = useCallback(async () => {
    const next = await toggleFavorite(mediaId);
    setIsFavorite(next);
  }, [mediaId]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isTypingTarget(e.target)) return;
      if (editorOpen) return;

      if (e.key === "Escape") {
        requestClose();
        return;
      }

      if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        if (window.confirm(t("viewer.confirmDelete"))) {
          void (async () => {
            try {
              await deleteMedia(mediaId);
              closeViewer();
              await loadMedia();
            } catch (error) {
              console.error("Delete failed:", error);
            }
          })();
        }
        return;
      }

      if (e.key === "f" || e.key === "F") {
        e.preventDefault();
        void handleToggleFavorite();
        return;
      }

      if (e.key === "i" || e.key === "I") {
        e.preventDefault();
        setShowInfo((v) => !v);
        return;
      }

      if ((e.key === "e" || e.key === "E") && media?.media_type !== "Video") {
        e.preventDefault();
        setEditorOpen(true);
        return;
      }

      if (media?.media_type === "Video") return;
      if (e.key === "r" || e.key === "R") {
        e.preventDefault();
        setRotation((r) => (e.shiftKey ? (r - 90 + 360) % 360 : (r + 90) % 360));
        return;
      }
      if (e.key === "ArrowLeft") {
        navigate(neighbors.prev_id);
      } else if (e.key === "ArrowRight") {
        navigate(neighbors.next_id);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [navigate, neighbors, media?.media_type, mediaId, editorOpen, handleToggleFavorite, requestClose, t]);

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

  const handleMainImageLoad = useCallback(() => {
    setCurrentImageLoaded(true);
  }, []);

  const handleShare = useCallback(async () => {
    if (!media || !navigator.share) return;
    try {
      await navigator.share({ title: media.filename, text: media.filename });
    } catch {
      // User cancelled or share unavailable
    }
  }, [media]);

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
  const imageTransform = [
    previewTransform,
    rotation !== 0 ? `rotate(${rotation}deg)` : undefined,
  ]
    .filter(Boolean)
    .join(" ");

  const isVideo = media?.media_type === "Video";
  const imageSrc = media
    ? useOriginal
      ? getOriginalUrl(media.path)
      : getThumbnailUrl(media.id, "large")
    : "";

  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal="true"
      aria-label={t("viewer.title")}
      tabIndex={-1}
      className={`flex min-h-0 flex-1 flex-col overflow-hidden bg-neutral-950 text-white outline-none ${
        closing ? "photo-viewer-exit" : "photo-viewer-enter"
      }`}
    >
      <div className="flex shrink-0 items-center gap-3 border-b border-white/10 px-4 py-2">
        <button
          type="button"
          onClick={requestClose}
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

        {!isVideo && rotation !== 0 && (
          <span
            className="shrink-0 rounded-full bg-white/10 px-2 py-0.5 text-xs font-medium text-neutral-300"
            title={t("viewer.rotation")}
          >
            {rotation}°
          </span>
        )}

        <p className="min-w-0 flex-1 truncate text-center text-sm font-medium text-neutral-100">
          {media ? formatMediaDate(media, locale, t("common.notAvailable")) : t("common.notAvailable")}
        </p>

        <div className="flex shrink-0 items-center gap-1">
          {edited && (
            <span className="rounded-full bg-blue-600/80 px-2 py-0.5 text-xs font-medium">
              {t("editor.hasEdits")}
            </span>
          )}
          {!isVideo && (
            <button
              type="button"
              onClick={() => {
                setShowSimilar((v) => {
                  if (!v) setShowInfo(false);
                  return !v;
                });
              }}
              className={`rounded-lg px-3 py-1.5 text-sm transition hover:bg-white/10 ${
                showSimilar ? "bg-white/10 text-white" : "text-neutral-300"
              }`}
              title={t("similar.findSimilar")}
              aria-label={t("similar.findSimilar")}
              aria-pressed={showSimilar}
            >
              ⊞
            </button>
          )}
          <button
            type="button"
            onClick={() => {
              setShowInfo((v) => {
                if (!v) setShowSimilar(false);
                return !v;
              });
            }}
            className={`rounded-lg px-3 py-1.5 text-sm transition hover:bg-white/10 ${
              showInfo ? "bg-white/10 text-white" : "text-neutral-300"
            }`}
            title={t("viewer.info")}
            aria-label={t("viewer.info")}
            aria-pressed={showInfo}
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
              aria-label={t("viewer.edit")}
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
                      onLoad={handleMainImageLoad}
                      style={{
                        transform: imageTransform || undefined,
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

            {showInfo && media && <InfoPanel media={media} />}
            {showSimilar && (
              <SimilarPhotosPanel mediaId={mediaId} onClose={() => setShowSimilar(false)} />
            )}
          </>
        )}
      </div>

      {!isVideo && (
        <div
          ref={filmstripRef}
          className="flex shrink-0 gap-2 overflow-x-auto border-t border-white/10 px-4 py-3"
          role="tablist"
          aria-label={t("viewer.title")}
        >
          {filmstrip.map((item) => (
            <button
              key={item.id}
              type="button"
              role="tab"
              aria-selected={item.id === mediaId}
              data-id={item.id}
              onClick={() => openViewer(item.id)}
              className={`h-16 w-16 shrink-0 overflow-hidden rounded-md transition ${
                item.id === mediaId ? "ring-2 ring-blue-500" : "opacity-70 hover:opacity-100"
              }`}
              aria-label={item.filename}
            >
              <img
                src={getThumbnailUrl(item.id, "small")}
                alt=""
                className="h-full w-full object-cover"
              />
            </button>
          ))}
        </div>
      )}

      <div className="shrink-0 border-t border-white/10 px-4 py-1.5 text-center text-xs text-neutral-400">
        <span className="truncate">{media?.filename ?? t("common.notAvailable")}</span>
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
