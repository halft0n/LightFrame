import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import {
  addToAlbum,
  getAlbumMedia,
  getMediaList,
  listAlbums,
  removeFromAlbum,
  type Album,
  type MediaItem,
} from "@/lib/tauri";
import { closeAlbumDetail, openViewer, useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 180;
const GAP = 12;
const PAGE_SIZE = 60;

export function AlbumDetailView() {
  const { t } = useTranslation();
  const { selectedAlbumId } = useAppStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const [album, setAlbum] = useState<Album | null>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [showPicker, setShowPicker] = useState(false);
  const [pickerMedia, setPickerMedia] = useState<MediaItem[]>([]);
  const [pickerSelected, setPickerSelected] = useState<Set<number>>(new Set());
  const [pickerLoading, setPickerLoading] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );

  const loadAlbum = useCallback(async () => {
    if (selectedAlbumId == null) return;
    setLoading(true);
    try {
      const [albums, items] = await Promise.all([
        listAlbums(),
        getAlbumMedia(selectedAlbumId, 0, PAGE_SIZE),
      ]);
      setAlbum(albums.find((a) => a.id === selectedAlbumId) ?? null);
      setMedia(items);
    } finally {
      setLoading(false);
    }
  }, [selectedAlbumId]);

  useEffect(() => {
    void loadAlbum();
  }, [loadAlbum]);

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

  const openPicker = async () => {
    setShowPicker(true);
    setPickerLoading(true);
    setPickerSelected(new Set());
    try {
      const items = await getMediaList(0, 200);
      const existing = new Set(media.map((m) => m.id));
      setPickerMedia(items.filter((m) => !existing.has(m.id)));
    } finally {
      setPickerLoading(false);
    }
  };

  const handleAddPhotos = async () => {
    if (selectedAlbumId == null || pickerSelected.size === 0) return;
    await addToAlbum(selectedAlbumId, [...pickerSelected]);
    setShowPicker(false);
    setPickerSelected(new Set());
    await loadAlbum();
  };

  const handleRemove = async (mediaId: number) => {
    if (selectedAlbumId == null) return;
    await removeFromAlbum(selectedAlbumId, mediaId);
    await loadAlbum();
  };

  if (selectedAlbumId == null) {
    return null;
  }

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  return (
    <div className="relative flex flex-1 flex-col overflow-hidden">
      <div className="flex items-center gap-3 border-b border-neutral-800 px-4 py-2">
        <button
          type="button"
          onClick={closeAlbumDetail}
          className="rounded-md px-2 py-1 text-sm text-neutral-400 transition hover:bg-white/10 hover:text-neutral-200"
        >
          ← {t("albums.back")}
        </button>
        <span className="text-sm font-medium text-neutral-200">
          {album?.name ?? t("albums.title")}
        </span>
        <span className="text-sm text-neutral-500">
          {t("gallery.count", { count: media.length })}
        </span>
        <div className="ml-auto">
          <button
            type="button"
            onClick={() => void openPicker()}
            className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white transition hover:bg-blue-500"
          >
            {t("albums.addPhotos")}
          </button>
        </div>
      </div>

      <div ref={parentRef} className="flex-1 overflow-y-auto px-4 py-3">
        {media.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-2 py-12 text-neutral-500">
            <p>{t("gallery.noPhotos")}</p>
            <button
              type="button"
              onClick={() => void openPicker()}
              className="text-sm text-blue-400 hover:text-blue-300"
            >
              {t("albums.addPhotos")}
            </button>
          </div>
        ) : (
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
                <button
                  type="button"
                  title={t("albums.removePhoto")}
                  onClick={() => void handleRemove(item.id)}
                  className="absolute right-2 top-2 rounded bg-black/70 px-2 py-0.5 text-xs text-white opacity-0 transition group-hover:opacity-100"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {showPicker && (
        <div className="absolute inset-0 z-20 flex items-center justify-center bg-black/60 p-4">
          <div className="flex max-h-[80vh] w-full max-w-3xl flex-col overflow-hidden rounded-lg border border-neutral-700 bg-neutral-900 shadow-xl">
            <div className="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
              <h3 className="text-sm font-medium text-neutral-200">{t("albums.addPhotos")}</h3>
              <button
                type="button"
                onClick={() => setShowPicker(false)}
                className="text-neutral-400 hover:text-neutral-200"
              >
                ✕
              </button>
            </div>
            <div className="flex-1 overflow-y-auto px-4 py-3">
              {pickerLoading ? (
                <p className="py-8 text-center text-neutral-500">{t("gallery.loading")}</p>
              ) : pickerMedia.length === 0 ? (
                <p className="py-8 text-center text-neutral-500">{t("gallery.noPhotos")}</p>
              ) : (
                <div className="grid grid-cols-4 gap-2 sm:grid-cols-5 md:grid-cols-6">
                  {pickerMedia.map((item) => {
                    const selected = pickerSelected.has(item.id);
                    return (
                      <button
                        key={item.id}
                        type="button"
                        onClick={() => {
                          setPickerSelected((prev) => {
                            const next = new Set(prev);
                            if (next.has(item.id)) next.delete(item.id);
                            else next.add(item.id);
                            return next;
                          });
                        }}
                        className={`aspect-square overflow-hidden rounded-md border-2 ${
                          selected ? "border-blue-500" : "border-transparent"
                        }`}
                      >
                        <PhotoCard
                          item={item}
                          selected={selected}
                          onSelect={() => {}}
                        />
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
            <div className="flex justify-end gap-2 border-t border-neutral-800 px-4 py-3">
              <button
                type="button"
                onClick={() => setShowPicker(false)}
                className="rounded-md px-3 py-1.5 text-sm text-neutral-400 hover:bg-neutral-800"
              >
                {t("viewer.close")}
              </button>
              <button
                type="button"
                disabled={pickerSelected.size === 0}
                onClick={() => void handleAddPhotos()}
                className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-500 disabled:opacity-50"
              >
                {t("albums.addPhotos")} ({pickerSelected.size})
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
