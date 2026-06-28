import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import {
  getSmartAlbumMedia,
  listSmartAlbums,
  type MediaItem,
  type SmartAlbum,
} from "@/lib/tauri";
import { closeSmartAlbumDetail, openViewer, useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 160;
const GAP = 3;
const PAGE_SIZE = 60;

export function SmartAlbumView() {
  const { t } = useTranslation();
  const { selectedSmartAlbumId } = useAppStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const [album, setAlbum] = useState<SmartAlbum | null>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );
  const hasMore = album != null && media.length < album.media_count;

  const loadInitial = useCallback(async () => {
    if (selectedSmartAlbumId == null) return;
    setLoading(true);
    try {
      const [albums, items] = await Promise.all([
        listSmartAlbums(),
        getSmartAlbumMedia(selectedSmartAlbumId, 0, PAGE_SIZE),
      ]);
      setAlbum(albums.find((a) => a.id === selectedSmartAlbumId) ?? null);
      setMedia(items);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, [selectedSmartAlbumId]);

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
    if (selectedSmartAlbumId == null || loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const items = await getSmartAlbumMedia(selectedSmartAlbumId, media.length, PAGE_SIZE);
      setMedia((prev) => [...prev, ...items]);
    } catch {
      // ignore
    } finally {
      setLoadingMore(false);
    }
  }, [hasMore, loadingMore, media.length, selectedSmartAlbumId]);

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

  if (selectedSmartAlbumId == null) return null;

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
          onClick={closeSmartAlbumDetail}
          className="rounded-md px-2 py-1 text-sm text-neutral-500 dark:text-neutral-400 transition hover:bg-white/10 hover:text-neutral-200"
        >
          ← {t("albums.back")}
        </button>
        <span className="text-sm font-medium text-neutral-200">
          {album?.icon && <span className="mr-1">{album.icon}</span>}
          {album?.name ?? t("smartAlbums.title")}
        </span>
        <span className="text-sm text-neutral-500">
          {t("gallery.count", { count: album?.media_count ?? media.length })}
        </span>
      </div>

      {media.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center text-neutral-500">
          <p>{t("gallery.noPhotos")}</p>
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
