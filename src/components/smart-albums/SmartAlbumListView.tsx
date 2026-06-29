import { useCallback, useEffect, useState } from "react";
import {
  createSmartAlbum,
  deleteSmartAlbum,
  listSmartAlbums,
  type SmartAlbum,
  type SmartAlbumRule,
} from "@/lib/tauri";
import { openSmartAlbumDetail } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

export function SmartAlbumListView() {
  const { t } = useTranslation();
  const [albums, setAlbums] = useState<SmartAlbum[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState("");
  const [creating, setCreating] = useState(false);

  const loadAlbums = useCallback(async () => {
    setLoading(true);
    try {
      setAlbums(await listSmartAlbums());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAlbums();
  }, [loadAlbums]);

  const handleCreate = async () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    setCreating(true);
    try {
      const rule: SmartAlbumRule = { media_type: "Photo" };
      await createSmartAlbum(trimmed, "✨", rule);
      setName("");
      setShowCreate(false);
      await loadAlbums();
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (e: React.MouseEvent, albumId: number) => {
    e.stopPropagation();
    if (!window.confirm(t("smartAlbums.confirmDelete"))) return;
    try {
      await deleteSmartAlbum(albumId);
      await loadAlbums();
    } catch (err) {
      console.error("Failed to delete smart album:", err);
    }
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
      <div className="flex items-center justify-between border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-3">
        <h2 className="text-sm font-medium text-neutral-200">{t("smartAlbums.title")}</h2>
        <button
          type="button"
          onClick={() => setShowCreate(true)}
          className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white transition hover:bg-blue-500"
        >
          {t("smartAlbums.create")}
        </button>
      </div>

      {showCreate && (
        <div className="border-b border-neutral-200/80 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-900/80 px-4 py-3">
          <div className="flex max-w-md flex-col gap-2">
            <label className="text-sm text-neutral-500 dark:text-neutral-400">{t("albums.nameLabel")}</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="rounded-md border border-neutral-700 bg-neutral-800 px-3 py-2 text-sm text-neutral-100 outline-none focus:border-blue-500"
              autoFocus
            />
            <div className="flex gap-2 pt-1">
              <button
                type="button"
                disabled={creating || !name.trim()}
                onClick={() => void handleCreate()}
                className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white transition hover:bg-blue-500 disabled:opacity-50"
              >
                {t("smartAlbums.create")}
              </button>
              <button
                type="button"
                onClick={() => {
                  setShowCreate(false);
                  setName("");
                }}
                className="rounded-md px-3 py-1.5 text-sm text-neutral-500 dark:text-neutral-400 transition hover:bg-neutral-100 dark:hover:bg-neutral-800"
              >
                {t("viewer.close")}
              </button>
            </div>
          </div>
        </div>
      )}

      {albums.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-neutral-500">
          <p>{t("smartAlbums.empty")}</p>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto px-1 py-1">
          <div className="grid gap-[3px] sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {albums.map((album) => (
              <div
                key={album.id}
                role="button"
                tabIndex={0}
                onClick={() => openSmartAlbumDetail(album.id)}
                onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") openSmartAlbumDetail(album.id); }}
                className="group relative flex cursor-pointer flex-col overflow-hidden rounded-lg border border-neutral-200/80 dark:border-neutral-800 bg-neutral-100 dark:bg-neutral-900/50 text-left transition hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800/80"
              >
                <div className="flex aspect-square w-full items-center justify-center bg-neutral-800 text-5xl">
                  {album.icon ?? "📂"}
                </div>
                <div className="flex items-start justify-between gap-2 p-3">
                  <div className="min-w-0">
                    <p className="truncate font-medium text-neutral-100">{album.name}</p>
                    <p className="text-sm text-neutral-500">
                      {t("gallery.count", { count: album.media_count })}
                    </p>
                  </div>
                  <button
                    type="button"
                    title={t("albums.delete")}
                    onClick={(e) => void handleDelete(e, album.id)}
                    className="shrink-0 rounded p-1 text-neutral-500 opacity-0 transition hover:bg-red-500/20 hover:text-red-400 group-hover:opacity-100"
                  >
                    ✕
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
