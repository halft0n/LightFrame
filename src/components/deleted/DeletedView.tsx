import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import {
  getDeletedMedia,
  permanentlyDelete,
  restoreMedia,
  type MediaItem,
} from "@/lib/tauri";
import { openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 180;
const GAP = 12;

export function DeletedView() {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );

  const loadDeleted = useCallback(async () => {
    setLoading(true);
    try {
      setMedia(await getDeletedMedia());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadDeleted();
  }, [loadDeleted]);

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

  const handleRestore = async (mediaId: number) => {
    await restoreMedia(mediaId);
    await loadDeleted();
  };

  const handlePermanentDelete = async (mediaId: number) => {
    await permanentlyDelete(mediaId);
    await loadDeleted();
  };

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
        <h2 className="text-sm font-medium text-neutral-200">{t("deleted.title")}</h2>
        <p className="mt-1 text-sm text-amber-500/90">{t("deleted.notice")}</p>
        {media.length > 0 && (
          <p className="mt-0.5 text-sm text-neutral-500">
            {t("gallery.count", { count: media.length })}
          </p>
        )}
      </div>

      {media.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-neutral-500">
          <p>{t("deleted.empty")}</p>
          <p className="text-sm text-neutral-600">{t("deleted.emptyHint")}</p>
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
              <div key={item.id} className="group relative">
                <PhotoCard
                  item={item}
                  selected={false}
                  onSelect={() => openViewer(item.id)}
                  onOpen={openViewer}
                />
                <div className="absolute inset-x-0 bottom-0 flex gap-1 bg-gradient-to-t from-black/90 to-transparent p-2 opacity-0 transition group-hover:opacity-100">
                  <button
                    type="button"
                    onClick={() => void handleRestore(item.id)}
                    className="flex-1 rounded bg-green-600/90 px-2 py-1 text-xs text-white hover:bg-green-500"
                  >
                    {t("deleted.restore")}
                  </button>
                  <button
                    type="button"
                    onClick={() => void handlePermanentDelete(item.id)}
                    className="flex-1 rounded bg-red-600/90 px-2 py-1 text-xs text-white hover:bg-red-500"
                  >
                    {t("deleted.permanentDelete")}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
