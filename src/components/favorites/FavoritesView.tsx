import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import { getFavorites, getFavoritesCount, type MediaItem } from "@/lib/tauri";
import { openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 180;
const GAP = 12;
const PAGE_SIZE = 60;

export function FavoritesView() {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );
  const hasMore = media.length < totalCount;

  const loadInitial = useCallback(async () => {
    setLoading(true);
    try {
      const [items, count] = await Promise.all([
        getFavorites(0, PAGE_SIZE),
        getFavoritesCount(),
      ]);
      setMedia(items);
      setTotalCount(count);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadInitial();
  }, [loadInitial]);

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
    if (loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const items = await getFavorites(media.length, PAGE_SIZE);
      setMedia((prev) => [...prev, ...items]);
    } finally {
      setLoadingMore(false);
    }
  }, [hasMore, loadingMore, media.length]);

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
      <div className="border-b border-neutral-800 px-4 py-3">
        <h2 className="text-sm font-medium text-neutral-200">{t("favorites.title")}</h2>
        {totalCount > 0 && (
          <p className="mt-0.5 text-sm text-neutral-500">
            {t("gallery.count", { count: totalCount })}
          </p>
        )}
      </div>

      {media.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-neutral-500">
          <p>{t("favorites.empty")}</p>
          <p className="text-sm text-neutral-600">{t("favorites.emptyHint")}</p>
        </div>
      ) : (
        <div ref={parentRef} className="flex-1 overflow-y-auto px-4 py-3">
          <div
            className="grid gap-3"
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
