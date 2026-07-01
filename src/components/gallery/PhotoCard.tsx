import { memo, useCallback, useRef, useState } from "react";
import type { MediaItem } from "@/lib/tauri";
import { getThumbnailUrl } from "@/lib/tauri";
import { dragMediaIdsForItem, setDragMediaIds } from "@/lib/dragMedia";
import { useTranslation } from "@/i18n/useTranslation";
import type { ThumbnailSize } from "@/store/appStore";
import type { ScrollIntent } from "@/hooks/useScrollIntent";
import { useLongPress } from "@/hooks/useLongPress";

interface PhotoCardProps {
  item: MediaItem;
  selected: boolean;
  selectedMediaIds?: number[];
  onSelect: (id: number, event: React.MouseEvent) => void;
  onOpen?: (id: number) => void;
  onPreview?: (id: number) => void;
  animationIndex?: number;
  thumbnailSize?: ThumbnailSize;
  scrollIntent?: ScrollIntent;
}

function thumbnailProtocolSize(
  uiSize: ThumbnailSize,
): "small" | "large" {
  return uiSize === "large" ? "large" : "small";
}

function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

function isHeicFilename(filename: string): boolean {
  const ext = filename.split(".").pop()?.toLowerCase();
  return ext === "heic" || ext === "heif";
}

export const PhotoCard = memo(function PhotoCard({
  item,
  selected,
  selectedMediaIds = [],
  onSelect,
  onOpen,
  onPreview,
  animationIndex = 0,
  thumbnailSize = "small",
  scrollIntent = "idle",
}: PhotoCardProps) {
  const { t } = useTranslation();
  const [microLoaded, setMicroLoaded] = useState(false);
  const [microError, setMicroError] = useState(false);
  const [fullLoaded, setFullLoaded] = useState(false);
  const [error, setError] = useState(false);

  const longPressFiredRef = useRef(false);

  const handleLongPress = useCallback(() => {
    longPressFiredRef.current = true;
    onPreview?.(item.id);
  }, [onPreview, item.id]);

  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      if (longPressFiredRef.current) {
        longPressFiredRef.current = false;
        return;
      }
      onSelect(item.id, e);
    },
    [item.id, onSelect],
  );

  const longPressHandlers = useLongPress(handleLongPress);

  const skipMicro = thumbnailSize === "large";
  const deferFull = scrollIntent === "fast" || scrollIntent === "burst";
  const isVideo = item.media_type === "Video";
  const isRaw = item.media_type === "Raw";
  const isLivePhoto = item.media_type === "LivePhoto";
  const isHeic = item.media_type === "Photo" && isHeicFilename(item.filename);

  const microUrl = getThumbnailUrl(item.id, "micro");
  const fullUrl = getThumbnailUrl(item.id, thumbnailProtocolSize(thumbnailSize));

  const handleMicroLoad = useCallback(() => {
    setMicroLoaded(true);
  }, []);

  const handleMicroError = useCallback(() => {
    setMicroLoaded(true);
    setMicroError(true);
  }, []);

  const handleFullLoad = useCallback(() => {
    setFullLoaded(true);
  }, []);

  const handleFullError = useCallback(() => {
    setError(true);
  }, []);

  const handleDragStart = useCallback(
    (e: React.DragEvent) => {
      setDragMediaIds(
        e.dataTransfer,
        dragMediaIdsForItem(item.id, selectedMediaIds),
      );
    },
    [item.id, selectedMediaIds],
  );

  const showSkeleton = !microLoaded && !fullLoaded && !error && !skipMicro;
  const shouldLoadFull = microLoaded && !deferFull;

  return (
    <button
      type="button"
      role="gridcell"
      aria-selected={selected}
      aria-label={item.filename}
      draggable
      onDragStart={handleDragStart}
      onClick={handleClick}
      onDoubleClick={() => onOpen?.(item.id)}
      onPointerDown={longPressHandlers.onPointerDown}
      onPointerUp={longPressHandlers.onPointerUp}
      onPointerMove={longPressHandlers.onPointerMove}
      onPointerCancel={longPressHandlers.onPointerCancel}
      style={{ "--stagger-index": animationIndex } as React.CSSProperties}
      className={`photo-card photo-card-enter relative aspect-square w-full overflow-hidden text-left ${
        selected ? "ring-2 ring-blue-500" : ""
      }`}
    >
      {showSkeleton && (
        <div className="photo-card-skeleton shimmer" aria-hidden="true" />
      )}

      {error ? (
        <div
          className="absolute inset-0 flex items-center justify-center bg-neutral-200 text-neutral-500 dark:bg-neutral-800"
          role="img"
          aria-label={t("a11y.thumbnailError")}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            className="h-10 w-10"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909M3.75 21h16.5A2.25 2.25 0 0022.5 18.75V5.25A2.25 2.25 0 0020.25 3H3.75A2.25 2.25 0 001.5 5.25v13.5A2.25 2.25 0 003.75 21z"
            />
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9 9.75h.008v.008H9V9.75z"
            />
          </svg>
        </div>
      ) : skipMicro ? (
        <img
          src={fullUrl}
          alt={item.filename}
          loading="lazy"
          decoding="async"
          onLoad={handleFullLoad}
          onError={handleFullError}
          className={`h-full w-full object-cover ${fullLoaded ? "photo-card-image-loaded opacity-100" : "opacity-0"}`}
        />
      ) : (
        <>
          <img
            src={microUrl}
            alt={item.filename}
            loading="lazy"
            decoding="async"
            onLoad={handleMicroLoad}
            onError={handleMicroError}
            className={`h-full w-full object-cover ${microError || fullLoaded ? "opacity-0 absolute inset-0" : "opacity-100"}`}
            style={{ imageRendering: "pixelated" }}
          />
          {shouldLoadFull && (
            <img
              src={fullUrl}
              alt={item.filename}
              loading="lazy"
              decoding="async"
              onLoad={handleFullLoad}
              onError={handleFullError}
              className={`absolute inset-0 h-full w-full object-cover ${fullLoaded ? "photo-card-image-loaded opacity-100" : "opacity-0"}`}
            />
          )}
        </>
      )}

      {isVideo && item.duration_sec != null && (
        <span className="absolute bottom-1 right-1 rounded bg-black/60 px-1 py-0.5 text-[10px] font-medium tabular-nums text-white">
          {formatDuration(item.duration_sec)}
        </span>
      )}

      {isLivePhoto && (
        <span className="absolute left-1 top-1 rounded bg-green-600 px-1 text-xs font-medium text-white">
          {t("gallery.liveBadge")}
        </span>
      )}

      {isRaw && !isLivePhoto && (
        <span className="absolute left-1 top-1 rounded bg-amber-600 px-1 text-xs font-medium text-white">
          {t("gallery.rawBadge")}
        </span>
      )}

      {isHeic && !isLivePhoto && (
        <span className="absolute left-1 top-1 rounded bg-sky-600 px-1 text-xs font-medium text-white">
          {t("gallery.heicBadge")}
        </span>
      )}

      {selected && (
        <span
          className="absolute left-1 top-1 h-2 w-2 rounded-full bg-blue-500 ring-2 ring-white dark:ring-neutral-950"
          aria-hidden="true"
        />
      )}
    </button>
  );
});
