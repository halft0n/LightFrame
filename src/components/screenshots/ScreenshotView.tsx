import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import {
  getScreenshotCount,
  getScreenshots,
  type MediaItem,
  type ScreenshotCategory,
} from "@/lib/tauri";
import { openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 160;
const GAP = 3;
const PAGE_SIZE = 60;

const CATEGORIES: ScreenshotCategory[] = [
  "all",
  "generic",
  "code",
  "chat",
  "document",
  "game",
  "webpage",
];

function categoryLabelKey(category: ScreenshotCategory): string {
  return category === "all"
    ? "screenshots.all"
    : `screenshots.category.${category}`;
}

export function ScreenshotView() {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const [category, setCategory] = useState<ScreenshotCategory>("all");
  const [screenshots, setScreenshots] = useState<MediaItem[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );
  const hasMore = screenshots.length < totalCount;

  const loadInitial = useCallback(async (selected: ScreenshotCategory) => {
    setLoading(true);
    try {
      const [items, count] = await Promise.all([
        getScreenshots(selected, 0, PAGE_SIZE),
        getScreenshotCount(selected),
      ]);
      setScreenshots(items);
      setTotalCount(count);
    } catch {
      setScreenshots([]);
      setTotalCount(0);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadInitial(category);
  }, [category, loadInitial]);

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
      const items = await getScreenshots(
        category,
        screenshots.length,
        PAGE_SIZE,
      );
      setScreenshots((prev) => [...prev, ...items]);
    } catch {
      // ignore
    } finally {
      setLoadingMore(false);
    }
  }, [category, hasMore, loadingMore, screenshots.length]);

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
      <div className="border-b border-neutral-200/80 dark:border-neutral-800 px-6 py-3">
        <h2 className="text-base font-semibold">{t("screenshots.title")}</h2>
        <p className="text-xs text-neutral-500 dark:text-neutral-400">
          {t("gallery.count", { count: totalCount })}
        </p>
        <div className="mt-3 flex flex-wrap gap-2">
          {CATEGORIES.map((item) => (
            <button
              key={item}
              type="button"
              onClick={() => setCategory(item)}
              className={`rounded-full px-3 py-1 text-xs font-medium transition ${
                category === item
                  ? "bg-blue-600 text-white"
                  : "bg-neutral-100 text-neutral-600 hover:bg-neutral-200 dark:bg-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-700"
              }`}
            >
              {t(categoryLabelKey(item))}
            </button>
          ))}
        </div>
      </div>

      {screenshots.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center py-20 text-neutral-500">
          <div className="text-5xl">📱</div>
          <p className="mt-4 text-lg">{t("screenshots.noScreenshots")}</p>
          <p className="mt-1 text-sm text-neutral-600">
            {t("screenshots.noScreenshotsHint")}
          </p>
        </div>
      ) : (
        <div ref={parentRef} className="flex-1 overflow-y-auto px-1 py-1">
          <div
            className="grid gap-[3px]"
            style={{
              gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
            }}
          >
            {screenshots.map((item) => (
              <PhotoCard
                key={item.id}
                item={item}
                selected={false}
                selectedMediaIds={[]}
                onSelect={() => openViewer(item.id)}
                onOpen={openViewer}
              />
            ))}
          </div>
          {loadingMore && (
            <div className="py-4 text-center text-sm text-neutral-500">
              {t("gallery.loading")}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
