import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import {
  getMemoryMedia,
  listMemories,
  type MediaItem,
  type Memory,
} from "@/lib/tauri";
import { closeMemoryDetail, openViewer, useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 160;
const GAP = 3;
const PAGE_SIZE = 60;

export function MemoryDetailView() {
  const { t } = useTranslation();
  const { selectedMemoryId } = useAppStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const [memory, setMemory] = useState<Memory | null>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );
  const hasMore = memory != null && media.length < memory.media_count;

  const loadInitial = useCallback(async () => {
    if (selectedMemoryId == null) return;
    setLoading(true);
    try {
      const [memories, items] = await Promise.all([
        listMemories(),
        getMemoryMedia(selectedMemoryId, 0, PAGE_SIZE),
      ]);
      setMemory(memories.find((m) => m.id === selectedMemoryId) ?? null);
      setMedia(items);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, [selectedMemoryId]);

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
    if (selectedMemoryId == null || loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const items = await getMemoryMedia(selectedMemoryId, media.length, PAGE_SIZE);
      setMedia((prev) => [...prev, ...items]);
    } catch {
      // ignore
    } finally {
      setLoadingMore(false);
    }
  }, [hasMore, loadingMore, media.length, selectedMemoryId]);

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

  if (selectedMemoryId == null) return null;

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex items-center gap-3 border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-2">
        <button
          type="button"
          onClick={closeMemoryDetail}
          className="rounded-md px-2 py-1 text-sm text-neutral-500 dark:text-neutral-400 transition hover:bg-white/10 hover:text-neutral-200"
        >
          ← {t("albums.back")}
        </button>
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-neutral-200">{memory?.title}</p>
          {memory?.subtitle && (
            <p className="truncate text-xs text-neutral-500">{memory.subtitle}</p>
          )}
        </div>
        <span className="ml-auto text-sm text-neutral-500">
          {t("memories.photos", { count: memory?.media_count ?? media.length })}
        </span>
      </div>

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
    </div>
  );
}
