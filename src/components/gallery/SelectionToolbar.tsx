import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  batchAddToAlbum,
  batchDeleteMedia,
  batchExport,
  batchToggleFavorite,
  listAlbums,
  removeFromAlbum,
  type Album,
} from "@/lib/tauri";
import {
  clearMediaSelection,
  loadMedia,
  startSlideshow,
  useAppStore,
} from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

export function SelectionToolbar({
  onAlbumChanged,
}: {
  onAlbumChanged?: () => void | Promise<void>;
}) {
  const { t } = useTranslation();
  const { selectedMediaIds, currentView, selectedAlbumId } = useAppStore();
  const [albums, setAlbums] = useState<Album[]>([]);
  const [showAlbumPicker, setShowAlbumPicker] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [exportMessage, setExportMessage] = useState<string | null>(null);

  const count = selectedMediaIds.length;
  const isTauri = Boolean(window.__TAURI_INTERNALS__);
  const inAlbumContext = currentView === "album-detail" && selectedAlbumId != null;

  useEffect(() => {
    if (count === 0) return;
    void listAlbums().then(setAlbums).catch(() => setAlbums([]));
  }, [count]);

  const refreshMedia = useCallback(async () => {
    try {
      await loadMedia();
    } catch (err) {
      console.error("Failed to refresh media:", err);
    }
  }, []);

  const handleDelete = async () => {
    setBusy(true);
    try {
      await batchDeleteMedia(selectedMediaIds);
      clearMediaSelection();
      await refreshMedia();
    } catch (err) {
      console.error("Failed to delete selected media:", err);
    } finally {
      setBusy(false);
      setShowDeleteConfirm(false);
    }
  };

  const handleFavorite = async () => {
    setBusy(true);
    try {
      await batchToggleFavorite(selectedMediaIds, true);
      clearMediaSelection();
    } catch (err) {
      console.error("Failed to favorite selected media:", err);
    } finally {
      setBusy(false);
    }
  };

  const handleAddToAlbum = async (albumId: number) => {
    setBusy(true);
    try {
      await batchAddToAlbum(albumId, selectedMediaIds);
      clearMediaSelection();
      setShowAlbumPicker(false);
    } catch (err) {
      console.error("Failed to add selected media to album:", err);
    } finally {
      setBusy(false);
    }
  };

  const handleRemoveFromAlbum = async () => {
    if (selectedAlbumId == null) return;
    setBusy(true);
    try {
      await Promise.all(
        selectedMediaIds.map((mediaId) => removeFromAlbum(selectedAlbumId, mediaId)),
      );
      clearMediaSelection();
      await onAlbumChanged?.();
    } catch (err) {
      console.error("Failed to remove selected media from album:", err);
    } finally {
      setBusy(false);
    }
  };

  const handleExport = async () => {
    if (!isTauri) {
      alert(t("settings.tauriOnly"));
      return;
    }

    const selected = await open({
      directory: true,
      multiple: false,
      title: t("batch.export"),
    });

    if (!selected || Array.isArray(selected)) return;

    setBusy(true);
    try {
      const exported = await batchExport(selectedMediaIds, selected);
      setExportMessage(t("batch.exportSuccess", { count: exported }));
      clearMediaSelection();
    } catch (err) {
      console.error("Failed to export selected media:", err);
      setExportMessage(t("batch.exportError"));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (!exportMessage) return;
    const timer = setTimeout(() => setExportMessage(null), 3000);
    return () => clearTimeout(timer);
  }, [exportMessage]);

  if (count === 0) return null;

  return (
    <>
      <div className="pointer-events-none fixed inset-x-0 bottom-6 z-40 flex justify-center px-4">
        <div
          role="toolbar"
          aria-label={t("a11y.selectionToolbar")}
          className="pointer-events-auto flex items-center gap-3 rounded-xl border border-neutral-700 bg-neutral-900/95 px-4 py-2.5 shadow-xl backdrop-blur-sm dark:border-neutral-700 dark:bg-neutral-900/95"
        >
          <span className="text-sm font-medium text-neutral-200">
            {t("batch.selected", { count })}
          </span>

          <div className="h-5 w-px bg-neutral-700" />

          <button
            type="button"
            disabled={busy}
            onClick={() => setShowDeleteConfirm(true)}
            className="rounded-md px-3 py-1.5 text-sm text-red-400 transition hover:bg-red-950/50 active:bg-red-950/70 disabled:opacity-50"
            aria-label={t("batch.delete")}
          >
            {t("batch.delete")}
          </button>

          {inAlbumContext ? (
            <button
              type="button"
              disabled={busy}
              onClick={() => void handleRemoveFromAlbum()}
              className="rounded-md px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-neutral-800 disabled:opacity-50"
            >
              {t("batch.removeFromAlbum")}
            </button>
          ) : (
            <div className="relative">
              <button
                type="button"
                disabled={busy}
                onClick={() => setShowAlbumPicker((v) => !v)}
                className="rounded-md px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-neutral-800 disabled:opacity-50"
              >
                {t("batch.addToAlbum")}
              </button>

              {showAlbumPicker && (
                <div className="absolute bottom-full left-0 mb-2 max-h-48 min-w-48 overflow-y-auto rounded-lg border border-neutral-700 bg-neutral-900 py-1 shadow-lg">
                  {albums.length === 0 ? (
                    <p className="px-3 py-2 text-xs text-neutral-500">{t("albums.empty")}</p>
                  ) : (
                    albums.map((album) => (
                      <button
                        key={album.id}
                        type="button"
                        onClick={() => void handleAddToAlbum(album.id)}
                        className="block w-full px-3 py-2 text-left text-sm text-neutral-200 hover:bg-neutral-800"
                      >
                        {album.name}
                      </button>
                    ))
                  )}
                </div>
              )}
            </div>
          )}

          <button
            type="button"
            disabled={busy}
            onClick={() => {
              startSlideshow(selectedMediaIds);
              clearMediaSelection();
            }}
            className="rounded-md px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-neutral-800 disabled:opacity-50"
          >
            {t("slideshow.start")}
          </button>

          <button
            type="button"
            disabled={busy}
            onClick={() => void handleFavorite()}
            className="rounded-md px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-neutral-800 disabled:opacity-50"
          >
            {t("batch.favorite")}
          </button>

          <button
            type="button"
            disabled={busy}
            onClick={() => void handleExport()}
            className="rounded-md px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-neutral-800 disabled:opacity-50"
          >
            {t("batch.export")}
          </button>

          <button
            type="button"
            disabled={busy}
            onClick={clearMediaSelection}
            className="rounded-md px-3 py-1.5 text-sm text-neutral-400 transition hover:bg-neutral-800 disabled:opacity-50"
          >
            {t("batch.cancelSelection")}
          </button>
        </div>
      </div>

      {showDeleteConfirm && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
          role="presentation"
          onClick={() => setShowDeleteConfirm(false)}
          onKeyDown={(e) => {
            if (e.key === "Escape") setShowDeleteConfirm(false);
          }}
        >
          <div
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="delete-confirm-title"
            aria-describedby="delete-confirm-hint"
            className="w-full max-w-sm rounded-xl border border-neutral-700 bg-neutral-900 p-5 shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <p id="delete-confirm-title" className="text-sm font-medium text-neutral-100">
              {t("batch.confirmDelete", { count })}
            </p>
            <p id="delete-confirm-hint" className="mt-2 text-xs text-neutral-400">
              {t("batch.confirmDeleteHint")}
            </p>
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setShowDeleteConfirm(false)}
                className="rounded-md px-4 py-2 text-sm text-neutral-400 transition hover:bg-neutral-800"
              >
                {t("batch.moveCancel")}
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => void handleDelete()}
                className="rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-red-500 disabled:opacity-50"
              >
                {t("batch.moveConfirm")}
              </button>
            </div>
          </div>
        </div>
      )}

      {exportMessage && (
        <div
          role="status"
          aria-live="polite"
          className="pointer-events-none fixed inset-x-0 bottom-20 z-50 flex justify-center px-4"
        >
          <div className="pointer-events-auto rounded-lg border border-neutral-700 bg-neutral-900 px-4 py-2 text-sm text-neutral-200 shadow-xl">
            {exportMessage}
          </div>
        </div>
      )}
    </>
  );
}
