import { useCallback, useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { PhotoCard } from "./PhotoCard";
import { SelectionToolbar } from "./SelectionToolbar";
import { batchDeleteMedia, batchToggleFavorite, type MediaItem } from "@/lib/tauri";
import { isTypingTarget } from "@/lib/keyboard";
import {
  clearMediaSelection,
  loadMedia,
  loadMoreMedia,
  openViewer,
  selectMediaRange,
  setMediaSelection,
  setSingleMediaSelection,
  setThumbnailSize,
  THUMBNAIL_WIDTHS,
  toggleMediaSelection,
  useAppStore,
  type ThumbnailSize,
} from "@/store/appStore";
import { EmptyState } from "@/components/ui/EmptyState";
import { useTranslation } from "@/i18n/useTranslation";

const GAP = 3;

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
              level === "small" ? "h-3 w-3" : level === "medium" ? "h-3.5 w-3.5" : "h-4 w-4"
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
  const {
    mediaItems: storeItems,
    totalCount: storeTotalCount,
    selectedMediaIds,
    thumbnailSize,
  } = useAppStore();
  const mediaItems = itemsProp ?? storeItems;
  const totalCount = totalCountProp ?? storeTotalCount;
  const parentRef = useRef<HTMLDivElement>(null);
  const lastSelectedRef = useRef<number | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);
  const [internalLoadingMore, setInternalLoadingMore] = useState(false);

  const loadingMore = loadingMoreProp ?? internalLoadingMore;
  const columnWidth = THUMBNAIL_WIDTHS[thumbnailSize];
  const rowHeight = columnWidth + GAP;

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (columnWidth + GAP)),
  );
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
    try {
      await loadMoreMedia();
    } catch (err) {
      console.error("Failed to load more photos:", err);
    } finally {
      setInternalLoadingMore(false);
    }
  }, [hasMore, loadingMore, onLoadMore]);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => rowHeight,
    overscan: 3,
  });

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

    el.addEventListener("scroll", handleScroll, { passive: true });
    return () => el.removeEventListener("scroll", handleScroll);
  }, [handleScroll]);

  const selectedSet = new Set(selectedMediaIds);

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
        void (async () => {
          await batchDeleteMedia(selectedMediaIds);
          clearMediaSelection();
          lastSelectedRef.current = null;
          await loadMedia();
        })();
        return;
      }

      if (e.key === "f" || e.key === "F") {
        e.preventDefault();
        void (async () => {
          await batchToggleFavorite(selectedMediaIds, true);
        })();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedMediaIds, mediaItems]);

  if (mediaItems.length === 0) {
    return <EmptyState variant="photos" title={t("gallery.noPhotos")} />;
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {showSizeControl && (
        <div className="flex shrink-0 justify-end px-3 py-2">
          <ThumbnailSizeControl size={thumbnailSize} onChange={setThumbnailSize} />
        </div>
      )}
      <div ref={parentRef} className="flex-1 overflow-y-auto px-1 py-1">
        <div
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
                        onSelect={handleSelect}
                        onOpen={openViewer}
                      />
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>

        {loadingMore && (
          <div className="flex items-center justify-center gap-2 py-6">
            <div className="loading-shimmer-bar shimmer" aria-hidden="true" />
            <span className="text-sm text-neutral-500">{t("gallery.loading")}</span>
          </div>
        )}
      </div>

      <SelectionToolbar />
    </div>
  );
}
