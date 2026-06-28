import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import {
  getPersonMedia,
  listPersons,
  renamePerson,
  type MediaItem,
  type Person,
} from "@/lib/tauri";
import { closePersonDetail, openViewer, useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 180;
const GAP = 12;
const PAGE_SIZE = 60;

export function PersonDetailView() {
  const { t } = useTranslation();
  const { selectedPersonId } = useAppStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const [person, setPerson] = useState<Person | null>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [name, setName] = useState("");
  const [editingName, setEditingName] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );
  const hasMore = person != null && media.length < person.face_count;

  const loadInitial = useCallback(async () => {
    if (selectedPersonId == null) return;
    setLoading(true);
    try {
      const [people, items] = await Promise.all([
        listPersons(),
        getPersonMedia(selectedPersonId, 0, PAGE_SIZE),
      ]);
      const found = people.find((p) => p.id === selectedPersonId) ?? null;
      setPerson(found);
      setName(found?.name ?? "");
      setMedia(items);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, [selectedPersonId]);

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
    if (selectedPersonId == null || loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const items = await getPersonMedia(selectedPersonId, media.length, PAGE_SIZE);
      setMedia((prev) => [...prev, ...items]);
    } catch {
      // ignore
    } finally {
      setLoadingMore(false);
    }
  }, [hasMore, loadingMore, media.length, selectedPersonId]);

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
    } catch {
      // ignore
    } finally {
      setEditingName(false);
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
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex items-center gap-3 border-b border-neutral-800 px-4 py-2">
        <button
          type="button"
          onClick={closePersonDetail}
          className="rounded-md px-2 py-1 text-sm text-neutral-400 transition hover:bg-white/10 hover:text-neutral-200"
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
          {t("people.faceCount", { count: person?.face_count ?? media.length })}
        </span>
      </div>

      <div ref={parentRef} className="flex-1 overflow-y-auto px-4 py-4">
        {media.length === 0 ? (
          <div className="flex flex-1 items-center justify-center py-12 text-neutral-500">
            <p>{t("gallery.noPhotos")}</p>
          </div>
        ) : (
          <div
            className="grid gap-3"
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
        )}
        {loadingMore && (
          <div className="py-4 text-center text-sm text-neutral-500">{t("gallery.loading")}</div>
        )}
      </div>
    </div>
  );
}
