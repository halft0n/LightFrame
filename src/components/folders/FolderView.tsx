import { useCallback, useEffect, useState } from "react";
import { PhotoGrid } from "@/components/gallery/PhotoGrid";
import { getMediaByFolder, getMediaCountByFolder, type MediaItem } from "@/lib/tauri";
import { useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";
import { EmptyState } from "@/components/ui/EmptyState";

const PAGE_SIZE = 60;

export function FolderView() {
  const { t } = useTranslation();
  const { selectedFolderId, selectedFolderPath } = useAppStore();
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);

  const loadInitial = useCallback(async () => {
    if (selectedFolderId == null) return;
    setLoading(true);
    try {
      const [items, count] = await Promise.all([
        getMediaByFolder(selectedFolderId, 0, PAGE_SIZE),
        getMediaCountByFolder(selectedFolderId),
      ]);
      setMedia(items);
      setTotalCount(count);
    } catch (err) {
      console.error("Failed to load folder media:", err);
      setMedia([]);
      setTotalCount(0);
    } finally {
      setLoading(false);
    }
  }, [selectedFolderId]);

  useEffect(() => {
    void loadInitial();
  }, [loadInitial]);

  const loadMore = useCallback(async () => {
    if (selectedFolderId == null || loadingMore || media.length >= totalCount) return;
    setLoadingMore(true);
    try {
      const items = await getMediaByFolder(selectedFolderId, media.length, PAGE_SIZE);
      setMedia((prev) => [...prev, ...items]);
    } catch (err) {
      console.error("Failed to load more folder media:", err);
    } finally {
      setLoadingMore(false);
    }
  }, [selectedFolderId, loadingMore, media.length, totalCount]);

  if (selectedFolderId == null) {
    return <EmptyState variant="photos" title={t("gallery.noPhotos")} />;
  }

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="border-b border-neutral-200/80 px-4 py-3 dark:border-neutral-800">
        <h2 className="text-sm font-medium text-neutral-900 dark:text-neutral-200">
          {t("folder.title")}
        </h2>
        {selectedFolderPath && (
          <p className="mt-0.5 truncate text-sm text-neutral-500" title={selectedFolderPath}>
            {selectedFolderPath}
          </p>
        )}
        {totalCount > 0 && (
          <p className="mt-0.5 text-sm text-neutral-500">
            {t("gallery.count", { count: totalCount })}
          </p>
        )}
      </div>

      {media.length === 0 ? (
        <EmptyState variant="photos" title={t("gallery.noPhotos")} />
      ) : (
        <PhotoGrid
          items={media}
          totalCount={totalCount}
          onLoadMore={loadMore}
          loadingMore={loadingMore}
        />
      )}
    </div>
  );
}
