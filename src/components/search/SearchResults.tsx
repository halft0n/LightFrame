import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import { searchMedia, searchMediaCount, type MediaItem } from "@/lib/tauri";
import { openViewer, useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 160;
const GAP = 3;
const PAGE_SIZE = 60;

export function SearchResults() {
  const { t } = useTranslation();
  const { searchQuery } = useAppStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );
  const hasMore = media.length < totalCount;

  const loadInitial = useCallback(async (query: string) => {
    const trimmed = query.trim();
    if (!trimmed) {
      setMedia([]);
      setTotalCount(0);
      return;
    }

    setLoading(true);
    try {
      const [items, count] = await Promise.all([
        searchMedia(trimmed, PAGE_SIZE, 0),
        searchMediaCount(trimmed),
      ]);
      setMedia(items);
      setTotalCount(count);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadInitial(searchQuery);
  }, [searchQuery, loadInitial]);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) setContainerWidth(entry.contentRect.width);
    });
    observer.observe(el);
    setContainerWidth(el.clientWidth);
    return () => observer.disconnect();
  }, []);

  const loadMore = useCallback(async () => {
    const trimmed = searchQuery.trim();
    if (loadingMore || !hasMore || !trimmed) return;
    setLoadingMore(true);
    try {
      const items = await searchMedia(trimmed, PAGE_SIZE, media.length);
      setMedia((prev) => [...prev, ...items]);
    } finally {
      setLoadingMore(false);
    }
  }, [hasMore, loadingMore, media.length, searchQuery]);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = el;
      if (scrollHeight - scrollTop - clientHeight < 200) {
        void loadMore();
      }
    };

    el.addEventListener("scroll", handleScroll, { passive: true });
    return () => el.removeEventListener("scroll", handleScroll);
  }, [loadMore]);

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-3">
        <h2 className="text-sm font-medium text-neutral-200">{t("search.results")}</h2>
        <p className="mt-0.5 text-sm text-neutral-500">
          {searchQuery.trim() && (
            <>
              <span className="text-neutral-500 dark:text-neutral-400">&ldquo;{searchQuery.trim()}&rdquo;</span>
              {" · "}
            </>
          )}
          {t("search.count", { count: totalCount })}
        </p>
      </div>

      {media.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-neutral-500">
          <p>{t("search.noResults")}</p>
          <p className="text-sm text-neutral-600">{t("search.noResultsHint")}</p>
        </div>
      ) : (
        <div ref={parentRef} className="flex-1 overflow-y-auto px-1 py-1">
          <div
            className="grid gap-[3px]"
            style={{
              gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
            }}
          >
            {media.map((item) => (
              <PhotoCard
                key={item.id}
                item={item}
                selected={false}
                onSelect={() => openViewer(item.id)}
                onOpen={openViewer}
              />
            ))}
          </div>
          {loadingMore && (
            <div className="py-4 text-center text-sm text-neutral-500">{t("gallery.loading")}</div>
          )}
        </div>
      )}
    </div>
  );
}
