import { useCallback, useEffect, useRef, useState, useMemo } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { PhotoCard } from "./PhotoCard";
import { SelectionToolbar } from "./SelectionToolbar";
import {
  batchDeleteMedia,
  batchToggleFavorite,
  type MediaItem,
} from "@/lib/tauri";
import { isTypingTarget } from "@/lib/keyboard";
import {
  clearMediaSelection,
  loadMedia,
  loadMoreMedia,
  openViewer,
  selectMediaRange,
  setMediaScrollIndex,
  setMediaSelection,
  setSingleMediaSelection,
  setThumbnailSize,
  startSlideshow,
  THUMBNAIL_WIDTHS,
  toggleMediaSelection,
  useAppStoreSelector,
  type ThumbnailSize,
} from "@/store/appStore";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingIndicator } from "@/components/ui/LoadingIndicator";
import { ErrorBanner } from "@/components/ui/ErrorBanner";
import { useTranslation } from "@/i18n/useTranslation";
import { useScrollIntent, type ScrollIntent } from "@/hooks/useScrollIntent";

const GAP = 3;
const SCROLL_LISTENER_OPTIONS: AddEventListenerOptions = { passive: true };

export interface PhotoGridProps {
  items?: MediaItem[];
  totalCount?: number;
  onLoadMore?: () => Promise<void>;
  loadingMore?: boolean;
  showSizeControl?: boolean;
}

function ThumbnailSizeControl({
  size,
  onChange,
}: {
  size: ThumbnailSize;
  onChange: (size: ThumbnailSize) => void;
}) {
  const { t } = useTranslation();
  const levels: ThumbnailSize[] = ["small", "medium", "large"];

  return (
    <div
      className="flex items-center gap-0.5 rounded-lg border border-neutral-200/80 bg-white/80 p-0.5 dark:border-neutral-700 dark:bg-neutral-900/80"
      role="group"
      aria-label={t("gallery.thumbnailSize")}
    >
      {levels.map((level) => (
        <button
          key={level}
          type="button"
          onClick={() => onChange(level)}
          title={t(`gallery.thumbnailSize.${level}`)}
          aria-label={t(`gallery.thumbnailSize.${level}`)}
          aria-pressed={size === level}
          className={`rounded-md px-2 py-1 transition ${
            size === level
              ? "bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-100"
              : "text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 dark:hover:bg-neutral-800 dark:hover:text-neutral-300"
          }`}
        >
          <svg
            viewBox="0 0 24 24"
            fill="currentColor"
            className={
              level === "small"
                ? "h-3 w-3"
                : level === "medium"
                  ? "h-3.5 w-3.5"
                  : "h-4 w-4"
            }
            aria-hidden="true"
          >
            <rect x="3" y="3" width="18" height="18" rx="2" />
          </svg>
        </button>
      ))}
    </div>
  );
}

export function PhotoGrid({
  items: itemsProp,
  totalCount: totalCountProp,
  onLoadMore,
  loadingMore: loadingMoreProp,
  showSizeControl = true,
}: PhotoGridProps = {}) {
  const { t } = useTranslation();
  const storeItems = useAppStoreSelector((s) => s.mediaItems);
  const storeTotalCount = useAppStoreSelector((s) => s.totalCount);
  const selectedMediaIds = useAppStoreSelector((s) => s.selectedMediaIds);
  const thumbnailSize = useAppStoreSelector((s) => s.thumbnailSize);
  const mediaLoadError = useAppStoreSelector((s) => s.mediaLoadError);
  const mediaItems = itemsProp ?? storeItems;
  const totalCount = totalCountProp ?? storeTotalCount;
  const useStoreScroll = itemsProp == null;
  const parentRef = useRef<HTMLDivElement>(null);
  const lastSelectedRef = useRef<number | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);
  const [internalLoadingMore, setInternalLoadingMore] = useState(false);
  const [loadMoreError, setLoadMoreError] = useState(false);

  const loadingMore = loadingMoreProp ?? internalLoadingMore;
  const columnWidth = THUMBNAIL_WIDTHS[thumbnailSize];

  const columnCount = Math.max(
    containerWidth > 0 && containerWidth < 768 ? 2 : 1,
    Math.floor((containerWidth + GAP) / (columnWidth + GAP)),
  );
  const cellWidth =
    containerWidth > 0 && columnCount > 0
      ? (containerWidth - (columnCount - 1) * GAP) / columnCount
      : columnWidth;
  const rowHeight = Math.ceil(cellWidth) + GAP;
  const rowCount = Math.ceil(mediaItems.length / columnCount);
  const hasMore = mediaItems.length < totalCount;

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        setContainerWidth(entry.contentRect.width);
      }
    });
    observer.observe(el);
    setContainerWidth(el.clientWidth);

    return () => observer.disconnect();
  }, []);

  const loadMore = useCallback(async () => {
    if (loadingMore || !hasMore) return;
    if (onLoadMore) {
      await onLoadMore();
      return;
    }
    setInternalLoadingMore(true);
    setLoadMoreError(false);
    try {
      await loadMoreMedia();
    } catch {
      setLoadMoreError(true);
    } finally {
      setInternalLoadingMore(false);
    }
  }, [hasMore, loadingMore, onLoadMore]);

  const scrollIntent = useScrollIntent(parentRef);

  const overscanCount = useMemo(() => {
    const map: Record<ScrollIntent, number> = {
      idle: 5,
      slow: 5,
      medium: 3,
      fast: 1,
      burst: 0,
    };
    return map[scrollIntent];
  }, [scrollIntent]);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => rowHeight,
    overscan: overscanCount,
  });

  const prevLayoutRef = useRef({ columnCount: 0, rowHeight: 0 });

  useEffect(() => {
    const prev = prevLayoutRef.current;
    if (prev.columnCount !== columnCount || prev.rowHeight !== rowHeight) {
      prevLayoutRef.current = { columnCount, rowHeight };
      rowVirtualizer.measure();
    }
  }, [columnCount, rowHeight, rowVirtualizer]);

  useEffect(() => {
    if (!useStoreScroll) return;

    const virtualItems = rowVirtualizer.getVirtualItems();
    if (virtualItems.length === 0) return;
    const center = virtualItems[Math.floor(virtualItems.length / 2)];
    const centerIndex = center.index * columnCount;
    const index = Math.min(centerIndex, Math.max(0, mediaItems.length - 1));

    const timeoutId = window.setTimeout(() => {
      setMediaScrollIndex(index);
    }, 200);

    return () => window.clearTimeout(timeoutId);
  }, [rowVirtualizer.range, columnCount, mediaItems.length, useStoreScroll]);

  const handleScroll = useCallback(() => {
    const el = parentRef.current;
    if (!el || loadingMore || !hasMore) return;

    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (distanceFromBottom < rowHeight * 2) {
      void loadMore();
    }
  }, [hasMore, loadMore, loadingMore, rowHeight]);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;

    el.addEventListener("scroll", handleScroll, SCROLL_LISTENER_OPTIONS);
    return () =>
      el.removeEventListener("scroll", handleScroll, SCROLL_LISTENER_OPTIONS);
  }, [handleScroll]);

  const selectedSet = useMemo(
    () => new Set(selectedMediaIds),
    [selectedMediaIds],
  );

  const handleSelect = useCallback(
    (id: number, event: React.MouseEvent) => {
      if (event.shiftKey && lastSelectedRef.current != null) {
        selectMediaRange(lastSelectedRef.current, id, mediaItems);
      } else if (event.ctrlKey || event.metaKey) {
        toggleMediaSelection(id);
        lastSelectedRef.current = id;
      } else {
        setSingleMediaSelection(id);
        lastSelectedRef.current = id;
      }
    },
    [mediaItems],
  );

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isTypingTarget(e.target)) return;

      if (e.key === "Escape" && selectedMediaIds.length > 0) {
        clearMediaSelection();
        lastSelectedRef.current = null;
        return;
      }

      if ((e.key === "a" || e.key === "A") && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        setMediaSelection(mediaItems.map((item) => item.id));
        return;
      }

      if (selectedMediaIds.length === 0) return;

      if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        if (selectedMediaIds.length > 0) {
          const msg = t("gallery.confirmBatchDelete", {
            count: selectedMediaIds.length,
          });
          if (window.confirm(msg)) {
            void (async () => {
              try {
                await batchDeleteMedia(selectedMediaIds);
                clearMediaSelection();
                lastSelectedRef.current = null;
                await loadMedia();
              } catch (error) {
                console.error("Batch delete failed:", error);
              }
            })();
          }
        }
        return;
      }

      if (e.key === "f" || e.key === "F") {
        e.preventDefault();
        void (async () => {
          try {
            await batchToggleFavorite(selectedMediaIds, true);
          } catch (error) {
            console.error("Batch favorite failed:", error);
          }
        })();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedMediaIds, mediaItems, t]);

  if (mediaLoadError && mediaItems.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-3 text-neutral-400">
        <ErrorBanner message={mediaLoadError} />
        <button
          type="button"
          onClick={() => void loadMedia()}
          className="rounded-lg bg-white/10 px-4 py-1.5 text-sm transition hover:bg-white/20"
        >
          {t("viewer.retry")}
        </button>
      </div>
    );
  }

  if (mediaItems.length === 0) {
    return <EmptyState variant="photos" title={t("gallery.noPhotos")} />;
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {showSizeControl && (
        <div className="flex shrink-0 items-center justify-end gap-2 px-3 py-2">
          <button
            type="button"
            onClick={() => {
              const ids = mediaItems
                .filter((m) => m.media_type !== "Video")
                .map((m) => m.id);
              if (ids.length > 0) startSlideshow(ids);
            }}
            className="rounded-lg border border-neutral-200/80 px-3 py-1.5 text-xs font-medium text-neutral-600 transition hover:bg-neutral-100 active:bg-neutral-200 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800 dark:active:bg-neutral-700"
            title={t("slideshow.startAll")}
            aria-label={t("slideshow.startAll")}
          >
            ▶ {t("slideshow.start")}
          </button>
          <ThumbnailSizeControl
            size={thumbnailSize}
            onChange={setThumbnailSize}
          />
        </div>
      )}
      <div ref={parentRef} className="flex-1 overflow-y-auto px-1 py-1">
        {containerWidth === 0 ? null : (
        <div
          role="grid"
          aria-label={t("gallery.gridLabel")}
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: "100%",
            position: "relative",
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualRow) => {
            const rowIndex = virtualRow.index;
            const startIndex = rowIndex * columnCount;

            return (
              <div
                key={virtualRow.key}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  height: `${virtualRow.size}px`,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
              >
                <div
                  className="grid gap-[3px]"
                  style={{
                    gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
                  }}
                >
                  {Array.from({ length: columnCount }, (_, colIndex) => {
                    const itemIndex = startIndex + colIndex;
                    const item = mediaItems[itemIndex];
                    if (!item) return <div key={colIndex} />;

                    return (
                      <PhotoCard
                        key={item.id}
                        item={item}
                        selected={selectedSet.has(item.id)}
                        selectedMediaIds={selectedMediaIds}
                        onSelect={handleSelect}
                        onOpen={openViewer}
                        animationIndex={colIndex}
                        thumbnailSize={thumbnailSize}
                        scrollIntent={scrollIntent}
                      />
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
        )}

        {loadingMore && (
          <LoadingIndicator className="py-6" label={t("a11y.loadingPhotos")} />
        )}

        {loadMoreError && !loadingMore && (
          <div className="px-3 py-4">
            <ErrorBanner
              message={t("gallery.loadError")}
              onRetry={() => void loadMore()}
              className="rounded-lg border"
            />
          </div>
        )}
      </div>

      <SelectionToolbar />
    </div>
  );
}
