import { useCallback, useEffect, useState } from "react";
import {
  createAlbum,
  deleteAlbum,
  getThumbnailUrl,
  listAlbums,
  updateAlbum,
  type Album,
} from "@/lib/tauri";
import { openAlbumDetail } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

export function AlbumListView() {
  const { t } = useTranslation();
  const [albums, setAlbums] = useState<Album[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [editingAlbumId, setEditingAlbumId] = useState<number | null>(null);
  const [editingName, setEditingName] = useState("");

  const loadAlbums = useCallback(async () => {
    setLoading(true);
    try {
      setAlbums(await listAlbums());
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
      await createAlbum(trimmed, description.trim() || null);
      setName("");
      setDescription("");
      setShowCreate(false);
      await loadAlbums();
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (e: React.MouseEvent, albumId: number) => {
    e.stopPropagation();
    if (!window.confirm(t("albums.confirmDelete"))) return;
    try {
      await deleteAlbum(albumId);
      await loadAlbums();
    } catch (err) {
      console.error("Failed to delete album:", err);
    }
  };

  const startRename = (e: React.MouseEvent, album: Album) => {
    e.stopPropagation();
    setEditingAlbumId(album.id);
    setEditingName(album.name);
  };

  const cancelRename = (e?: React.MouseEvent) => {
    e?.stopPropagation();
    setEditingAlbumId(null);
    setEditingName("");
  };

  const saveRenameForAlbum = async (album: Album) => {
    const trimmed = editingName.trim();
    if (!trimmed) return;
    await updateAlbum(album.id, trimmed, album.description);
    setEditingAlbumId(null);
    setEditingName("");
    await loadAlbums();
  };

  const saveRename = async (e: React.MouseEvent, album: Album) => {
    e.stopPropagation();
    await saveRenameForAlbum(album);
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
        <h2 className="text-sm font-medium text-neutral-700 dark:text-neutral-200">{t("albums.title")}</h2>
        <button
          type="button"
          onClick={() => setShowCreate(true)}
          className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white transition hover:bg-blue-500"
        >
          {t("albums.create")}
        </button>
      </div>

      {showCreate && (
        <div className="border-b border-neutral-200/80 dark:border-neutral-800 bg-neutral-50 dark:bg-neutral-900/80 px-4 py-3">
          <div className="flex max-w-md flex-col gap-2">
            <label htmlFor="album-create-name" className="text-sm text-neutral-500 dark:text-neutral-400">
              {t("albums.nameLabel")}
            </label>
            <input
              id="album-create-name"
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
                {t("albums.create")}
              </button>
              <button
                type="button"
                onClick={() => {
                  setShowCreate(false);
                  setName("");
                  setDescription("");
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
          <p>{t("albums.empty")}</p>
          <p className="text-sm text-neutral-600">{t("albums.emptyHint")}</p>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto px-1 py-1">
          <div className="grid gap-[3px] sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {albums.map((album) => (
              <div
                key={album.id}
                role="button"
                tabIndex={0}
                onClick={() => {
                  if (editingAlbumId === album.id) return;
                  openAlbumDetail(album.id);
                }}
                onKeyDown={(e) => {
                  if (editingAlbumId === album.id) return;
                  if (e.key === "Enter" || e.key === " ") openAlbumDetail(album.id);
                }}
                className="group relative flex cursor-pointer flex-col overflow-hidden rounded-lg border border-neutral-200/80 dark:border-neutral-800 bg-neutral-100 dark:bg-neutral-900/50 text-left transition hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800/80"
              >
                <div className="aspect-square w-full bg-neutral-800">
                  {album.cover_media_id != null ? (
                    <img
                      src={getThumbnailUrl(album.cover_media_id, "small")}
                      alt=""
                      className="h-full w-full object-cover transition group-hover:scale-105"
                      loading="lazy"
                    />
                  ) : (
                    <div className="flex h-full items-center justify-center text-4xl text-neutral-600">
                      📁
                    </div>
                  )}
                </div>
                <div className="flex items-start justify-between gap-2 p-3">
                  <div className="min-w-0 flex-1">
                    {editingAlbumId === album.id ? (
                      <div
                        className="flex flex-col gap-1"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <input
                          type="text"
                          value={editingName}
                          onChange={(e) => setEditingName(e.target.value)}
                          onKeyDown={(e) => {
                            e.stopPropagation();
                            if (e.key === "Enter") void saveRenameForAlbum(album);
                            if (e.key === "Escape") cancelRename();
                          }}
                          className="w-full rounded-md border border-neutral-700 bg-neutral-800 px-2 py-1 text-sm text-neutral-100 outline-none focus:border-blue-500"
                          autoFocus
                        />
                        <div className="flex gap-1">
                          <button
                            type="button"
                            disabled={!editingName.trim()}
                            onClick={(e) => void saveRename(e, album)}
                            className="rounded bg-blue-600 px-2 py-0.5 text-xs text-white hover:bg-blue-500 disabled:opacity-50"
                          >
                            {t("people.rename")}
                          </button>
                          <button
                            type="button"
                            onClick={cancelRename}
                            className="rounded px-2 py-0.5 text-xs text-neutral-500 hover:bg-neutral-800"
                          >
                            {t("viewer.close")}
                          </button>
                        </div>
                      </div>
                    ) : (
                      <>
                        <p className="truncate font-medium text-neutral-800 dark:text-neutral-100">{album.name}</p>
                        <p className="text-sm text-neutral-500">
                          {t("gallery.count", { count: album.media_count })}
                        </p>
                      </>
                    )}
                  </div>
                  {editingAlbumId !== album.id && (
                    <div className="flex shrink-0 gap-0.5 opacity-0 transition group-hover:opacity-100">
                      <button
                        type="button"
                        title={t("albums.rename")}
                        onClick={(e) => startRename(e, album)}
                        className="rounded p-1 text-neutral-500 transition hover:bg-neutral-200/80 hover:text-neutral-700 dark:hover:bg-neutral-700 dark:hover:text-neutral-200"
                      >
                        ✎
                      </button>
                      <button
                        type="button"
                        title={t("albums.delete")}
                        onClick={(e) => void handleDelete(e, album.id)}
                        className="rounded p-1 text-neutral-500 transition hover:bg-red-500/20 hover:text-red-400"
                      >
                        ✕
                      </button>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
