import { useCallback, useEffect, useRef, useState } from "react";
import {
  getMediaById,
  getMediaList,
  getMediaNeighbors,
  getOriginalUrl,
  getThumbnailUrl,
  type MediaItem,
} from "@/lib/tauri";
import { closeViewer, openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";
import { VideoPlayer } from "./VideoPlayer";

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
  const dragStart = useRef({ x: 0, y: 0, panX: 0, panY: 0 });
  const filmstripRef = useRef<HTMLDivElement>(null);

  const resetView = useCallback(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
    setUseOriginal(false);
  }, []);

  useEffect(() => {
    resetView();
    let cancelled = false;

    void (async () => {
      const [item, nb, list] = await Promise.all([
        getMediaById(mediaId),
        getMediaNeighbors(mediaId),
        getMediaList(0, FILMSTRIP_SIZE * 3),
      ]);
      if (cancelled) return;
      setMedia(item);
      setNeighbors(nb);
      setFilmstrip(list);
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

  const isVideo = media?.media_type === "Video";
  const imageSrc = media
    ? useOriginal
      ? getOriginalUrl(media.path)
      : getThumbnailUrl(media.id, "large")
    : "";

  return (
    <div className="flex h-full flex-col bg-black/95 text-white">
      <div className="flex items-center justify-between px-4 py-3">
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => navigate(neighbors.prev_id)}
            disabled={neighbors.prev_id == null}
            className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10 disabled:opacity-30"
            title={t("viewer.prev")}
            aria-label={t("viewer.prev")}
          >
            ‹
          </button>
          <button
            type="button"
            onClick={() => navigate(neighbors.next_id)}
            disabled={neighbors.next_id == null}
            className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10 disabled:opacity-30"
            title={t("viewer.next")}
            aria-label={t("viewer.next")}
          >
            ›
          </button>
        </div>

        <div className="flex items-center gap-2">
          {!isVideo && (
            <>
              <button
                type="button"
                onClick={() => setZoom((z) => Math.min(MAX_ZOOM, z + 0.25))}
                className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10"
                title={t("viewer.zoomIn")}
              >
                +
              </button>
              <button
                type="button"
                onClick={() => {
                  setZoom((z) => {
                    const next = Math.max(MIN_ZOOM, z - 0.25);
                    if (next <= 1) setPan({ x: 0, y: 0 });
                    return next;
                  });
                }}
                className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10"
                title={t("viewer.zoomOut")}
              >
                −
              </button>
            </>
          )}
          <button
            type="button"
            onClick={() => setShowInfo((v) => !v)}
            className={`rounded-lg px-3 py-1.5 text-sm transition hover:bg-white/10 ${
              showInfo ? "bg-white/10 text-white" : "text-neutral-300"
            }`}
          >
            {t("viewer.info")}
          </button>
          <button
            type="button"
            onClick={closeViewer}
            className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10"
            title={t("viewer.close")}
            aria-label={t("viewer.close")}
          >
            ✕
          </button>
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
                  <img
                    src={imageSrc}
                    alt={media.filename}
                    draggable={false}
                    style={{
                      transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
                      transition: dragging ? "none" : "transform 0.1s ease-out",
                      maxHeight: "100%",
                      maxWidth: "100%",
                      objectFit: "contain",
                    }}
                    className="select-none"
                  />
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
          className="flex gap-2 overflow-x-auto border-t border-white/10 px-4 py-3"
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
    </div>
  );
}
