import { useCallback, useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { PhotoCard } from "./PhotoCard";
import { SelectionToolbar } from "./SelectionToolbar";
import { getMediaList } from "@/lib/tauri";
import {
  appendMedia,
  clearMediaSelection,
  openViewer,
  selectMediaRange,
  setSingleMediaSelection,
  toggleMediaSelection,
  useAppStore,
} from "@/store/appStore";
import { EmptyState } from "@/components/ui/EmptyState";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 160;
const GAP = 3;
const ROW_HEIGHT = MIN_COLUMN_WIDTH + GAP;
const PAGE_SIZE = 60;

export function PhotoGrid() {
  const { t } = useTranslation();
  const { mediaItems, totalCount, selectedMediaIds } = useAppStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const lastSelectedRef = useRef<number | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);
  const [loadingMore, setLoadingMore] = useState(false);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
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
    setLoadingMore(true);
    try {
      const items = await getMediaList(mediaItems.length, PAGE_SIZE);
      appendMedia(items);
    } catch (err) {
      console.error("Failed to load more photos:", err);
    } finally {
      setLoadingMore(false);
    }
  }, [hasMore, loadingMore, mediaItems.length]);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 3,
  });

  const handleScroll = useCallback(() => {
    const el = parentRef.current;
    if (!el || loadingMore || !hasMore) return;

    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (distanceFromBottom < ROW_HEIGHT * 2) {
      void loadMore();
    }
  }, [hasMore, loadMore, loadingMore]);

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
        selectMediaRange(lastSelectedRef.current, id);
      } else if (event.ctrlKey || event.metaKey) {
        toggleMediaSelection(id);
        lastSelectedRef.current = id;
      } else {
        setSingleMediaSelection(id);
        lastSelectedRef.current = id;
      }
    },
    [],
  );

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && selectedMediaIds.length > 0) {
        clearMediaSelection();
        lastSelectedRef.current = null;
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedMediaIds.length]);

  if (mediaItems.length === 0) {
    return <EmptyState variant="photos" title={t("gallery.noPhotos")} />;
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
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
