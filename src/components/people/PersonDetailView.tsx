import { useCallback, useEffect, useRef, useState } from "react";
import {
  getFaceThumbnailUrl,
  getPersonFaces,
  listPersons,
  renamePerson,
  splitFaceFromPerson,
  type FaceInfo,
  type Person,
} from "@/lib/tauri";
import { closePersonDetail, openViewer, useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 120;
const GAP = 3;
const PAGE_SIZE = 60;

export function PersonDetailView() {
  const { t } = useTranslation();
  const { selectedPersonId } = useAppStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const [person, setPerson] = useState<Person | null>(null);
  const [faces, setFaces] = useState<FaceInfo[]>([]);
  const [name, setName] = useState("");
  const [editingName, setEditingName] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [splittingId, setSplittingId] = useState<number | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );

  const loadInitial = useCallback(async () => {
    if (selectedPersonId == null) return;
    setLoading(true);
    setError(null);
    try {
      const [people, faceList] = await Promise.all([
        listPersons(),
        getPersonFaces(selectedPersonId, 0, PAGE_SIZE),
      ]);
      const found = people.find((p) => p.id === selectedPersonId) ?? null;
      setPerson(found);
      setName(found?.name ?? "");
      setFaces(faceList);
      setHasMore(faceList.length === PAGE_SIZE);
    } catch (err) {
      console.error("Failed to load person detail:", err);
      setHasMore(false);
      setError(t("errors.generic"));
    } finally {
      setLoading(false);
    }
  }, [selectedPersonId, t]);

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

  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), 3000);
    return () => clearTimeout(timer);
  }, [toast]);

  const loadMore = useCallback(async () => {
    if (selectedPersonId == null || loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const items = await getPersonFaces(selectedPersonId, faces.length, PAGE_SIZE);
      setFaces((prev) => [...prev, ...items]);
      setHasMore(items.length === PAGE_SIZE);
    } catch (err) {
      console.error("Failed to load more faces:", err);
    } finally {
      setLoadingMore(false);
    }
  }, [faces.length, hasMore, loadingMore, selectedPersonId]);

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

  const saveName = async () => {
    if (selectedPersonId == null) return;
    const trimmed = name.trim();
    try {
      await renamePerson(selectedPersonId, trimmed);
      setPerson((prev) => (prev ? { ...prev, name: trimmed || null } : prev));
    } catch (err) {
      console.error("Failed to rename person:", err);
    } finally {
      setEditingName(false);
    }
  };

  const handleSplitFace = async (faceId: number) => {
    setSplittingId(faceId);
    try {
      await splitFaceFromPerson(faceId);
      setFaces((prev) => prev.filter((f) => f.id !== faceId));
      setPerson((prev) =>
        prev ? { ...prev, face_count: Math.max(0, prev.face_count - 1) } : prev,
      );
      setToast(t("people.faceSplitSuccess"));
    } catch (err) {
      console.error("Failed to split face from person:", err);
    } finally {
      setSplittingId(null);
    }
  };

  if (selectedPersonId == null) return null;

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  return (
    <div className="relative flex flex-1 flex-col overflow-hidden">
      {toast && (
        <div className="pointer-events-none absolute bottom-4 left-1/2 z-50 -translate-x-1/2 rounded-lg bg-neutral-800 px-4 py-2 text-sm text-neutral-100 shadow-lg">
          {toast}
        </div>
      )}

      <div className="flex items-center gap-3 border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-2">
        <button
          type="button"
          onClick={closePersonDetail}
          className="rounded-md px-2 py-1 text-sm text-neutral-500 dark:text-neutral-400 transition hover:bg-white/10 hover:text-neutral-200"
        >
          ← {t("people.back")}
        </button>

        <div className="min-w-0 flex-1">
          {editingName ? (
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void saveName();
                  if (e.key === "Escape") {
                    setName(person?.name ?? "");
                    setEditingName(false);
                  }
                }}
                placeholder={t("people.unnamed")}
                className="w-full max-w-xs rounded-md border border-neutral-700 bg-neutral-800 px-2 py-1 text-sm text-neutral-100 outline-none focus:border-blue-500"
                autoFocus
              />
              <button
                type="button"
                onClick={() => void saveName()}
                className="rounded-md bg-blue-600 px-2 py-1 text-xs text-white hover:bg-blue-500"
              >
                {t("people.rename")}
              </button>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => setEditingName(true)}
              className="truncate text-sm font-medium text-neutral-200 hover:text-white"
              title={t("people.rename")}
            >
              {person?.name ?? t("people.unnamed")}
            </button>
          )}
        </div>

        <span className="text-sm text-neutral-500">
          {t("people.faceCount", { count: person?.face_count ?? faces.length })}
        </span>
      </div>

      {error && (
        <div className="border-b border-red-900/50 bg-red-950/30 px-4 py-2">
          <p className="text-sm text-red-400">{error}</p>
        </div>
      )}

      <div ref={parentRef} className="flex-1 overflow-y-auto px-1 py-1">
        {faces.length === 0 ? (
          <div className="flex flex-1 items-center justify-center py-12 text-neutral-500">
            <p>{t("gallery.noPhotos")}</p>
          </div>
        ) : (
          <div
            className="grid gap-[3px]"
            style={{
              gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
            }}
          >
            {faces.map((face) => (
              <div
                key={face.id}
                className="group relative aspect-square overflow-hidden rounded-md bg-neutral-800"
              >
                <button
                  type="button"
                  onClick={() => openViewer(face.media_id)}
                  className="block h-full w-full"
                  title={t("people.splitFace")}
                >
                  <img
                    src={getFaceThumbnailUrl(face.id)}
                    alt=""
                    className="h-full w-full object-cover"
                    loading="lazy"
                    decoding="async"
                  />
                </button>
                <button
                  type="button"
                  onClick={() => void handleSplitFace(face.id)}
                  disabled={splittingId === face.id}
                  className="absolute bottom-1 left-1 right-1 rounded bg-black/70 px-2 py-1 text-xs text-neutral-200 opacity-0 transition group-hover:opacity-100 hover:bg-red-900/80 disabled:opacity-50"
                >
                  {splittingId === face.id
                    ? t("gallery.loading")
                    : t("people.removeFromPerson")}
                </button>
              </div>
            ))}
          </div>
        )}
        {loadingMore && (
          <div className="py-4 text-center text-sm text-neutral-500">{t("gallery.loading")}</div>
        )}
      </div>
    </div>
  );
}
