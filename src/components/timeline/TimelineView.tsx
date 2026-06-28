import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import { getTimelineGroups, type MediaItem, type TimelineGroup } from "@/lib/tauri";
import { openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 180;
const GAP = 12;
const HEADER_HEIGHT = 52;
const ROW_HEIGHT = MIN_COLUMN_WIDTH + GAP;
const PAGE_SIZE = 200;
const SCROLL_THRESHOLD = 200;

type VirtualRow =
  | { type: "header"; date: string; count: number; key: string }
  | { type: "grid-row"; date: string; items: MediaItem[]; key: string };

function isToday(dateStr: string): boolean {
  const today = new Date();
  const d = new Date(dateStr + "T00:00:00");
  return (
    d.getFullYear() === today.getFullYear() &&
    d.getMonth() === today.getMonth() &&
    d.getDate() === today.getDate()
  );
}

function isYesterday(dateStr: string): boolean {
  const yesterday = new Date();
  yesterday.setDate(yesterday.getDate() - 1);
  const d = new Date(dateStr + "T00:00:00");
  return (
    d.getFullYear() === yesterday.getFullYear() &&
    d.getMonth() === yesterday.getMonth() &&
    d.getDate() === yesterday.getDate()
  );
}

function formatDateHeader(dateStr: string, locale: string, t: (key: string) => string): string {
  if (isToday(dateStr)) return t("timeline.today");
  if (isYesterday(dateStr)) return t("timeline.yesterday");

  const date = new Date(dateStr + "T00:00:00");
  if (locale === "zh-CN") {
    const weekday = new Intl.DateTimeFormat("zh-CN", { weekday: "long" }).format(date);
    const year = date.getFullYear();
    const month = date.getMonth() + 1;
    const day = date.getDate();
    return `${year}年${month}月${day}日 ${weekday}`;
  }

  return new Intl.DateTimeFormat("en-US", {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(date);
}

function buildVirtualRows(groups: TimelineGroup[], columnCount: number): VirtualRow[] {
  const rows: VirtualRow[] = [];

  for (const group of groups) {
    rows.push({
      type: "header",
      date: group.date,
      count: group.count,
      key: `header-${group.date}`,
    });

    const gridRowCount = Math.ceil(group.media.length / columnCount);
    for (let rowIndex = 0; rowIndex < gridRowCount; rowIndex++) {
      const start = rowIndex * columnCount;
      rows.push({
        type: "grid-row",
        date: group.date,
        items: group.media.slice(start, start + columnCount),
        key: `row-${group.date}-${rowIndex}`,
      });
    }
  }

  return rows;
}

export function TimelineView() {
  const { t, locale } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const [groups, setGroups] = useState<TimelineGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [offset, setOffset] = useState(0);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void getTimelineGroups(PAGE_SIZE, 0).then((data) => {
      if (!cancelled) {
        const itemCount = data.reduce((sum, g) => sum + g.media.length, 0);
        setGroups(data);
        setHasMore(itemCount >= PAGE_SIZE);
        setOffset(itemCount);
        setLoading(false);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const loadMore = useCallback(async () => {
    if (loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const data = await getTimelineGroups(PAGE_SIZE, offset);
      const itemCount = data.reduce((sum, g) => sum + g.media.length, 0);
      setGroups((prev) => {
        if (prev.length === 0) return data;
        if (data.length === 0) return prev;

        const lastGroup = prev[prev.length - 1];
        const firstNew = data[0];

        if (lastGroup.date === firstNew.date) {
          const merged = [...prev];
          merged[merged.length - 1] = {
            ...lastGroup,
            count: lastGroup.count + firstNew.count,
            media: [...lastGroup.media, ...firstNew.media],
          };
          return [...merged, ...data.slice(1)];
        }

        return [...prev, ...data];
      });
      setHasMore(itemCount >= PAGE_SIZE);
      setOffset((prev) => prev + itemCount);
    } finally {
      setLoadingMore(false);
    }
  }, [loadingMore, hasMore, offset]);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = el;
      if (scrollHeight - scrollTop - clientHeight < SCROLL_THRESHOLD) {
        void loadMore();
      }
    };

    el.addEventListener("scroll", handleScroll);
    return () => el.removeEventListener("scroll", handleScroll);
  }, [loadMore]);

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

  const virtualRows = useMemo(
    () => buildVirtualRows(groups, columnCount),
    [groups, columnCount],
  );

  const getRowHeight = useCallback(
    (index: number) => {
      const row = virtualRows[index];
      if (!row) return HEADER_HEIGHT;
      return row.type === "header" ? HEADER_HEIGHT : ROW_HEIGHT;
    },
    [virtualRows],
  );

  const rowVirtualizer = useVirtualizer({
    count: virtualRows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: getRowHeight,
    overscan: 4,
  });

  const totalPhotos = groups.reduce((sum, g) => sum + g.count, 0);

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  if (groups.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("timeline.noPhotos")}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="border-b border-neutral-800 px-4 py-2 text-sm text-neutral-400">
        {t("timeline.title")} · {t("gallery.count", { count: totalPhotos })}
      </div>

      <div ref={parentRef} className="flex-1 overflow-y-auto px-4 py-3">
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: "100%",
            position: "relative",
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualRow) => {
            const row = virtualRows[virtualRow.index];
            if (!row) return null;

            if (row.type === "header") {
              return (
                <div
                  key={row.key}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  <div className="sticky top-0 z-10 flex items-center gap-3 bg-neutral-950/95 py-3 backdrop-blur-sm">
                    <h2 className="text-base font-semibold text-neutral-100">
                      {formatDateHeader(row.date, locale, t)}
                    </h2>
                    <span className="text-sm text-neutral-500">
                      {t("gallery.count", { count: row.count })}
                    </span>
                  </div>
                </div>
              );
            }

            return (
              <div
                key={row.key}
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
                  className="grid gap-3"
                  style={{
                    gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
                  }}
                >
                  {row.items.map((item) => (
                    <PhotoCard
                      key={item.id}
                      item={item}
                      selected={false}
                      onSelect={() => openViewer(item.id)}
                      onOpen={openViewer}
                    />
                  ))}
                  {Array.from({ length: columnCount - row.items.length }, (_, i) => (
                    <div key={`empty-${i}`} />
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
