import { useCallback, useEffect, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import {
  searchMedia,
  searchMediaCount,
  semanticSearch,
  getAiStatus,
  type MediaItem,
  type SearchResult,
  type AiStatus,
} from "@/lib/tauri";
import { openViewer, setSearchMode, useAppStore, type SearchMode } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const MIN_COLUMN_WIDTH = 160;
const GAP = 3;
const PAGE_SIZE = 60;

function searchResultToMediaItem(result: SearchResult): MediaItem {
  return {
    id: result.media_id,
    path: result.file_path,
    filename: result.file_name,
    media_type: "Photo",
    size_bytes: 0,
    modified_at: new Date().toISOString(),
  };
}

interface SearchModeToggleProps {
  mode: SearchMode;
  onChange: (mode: SearchMode) => void;
}

function SearchModeToggle({ mode, onChange }: SearchModeToggleProps) {
  const { t } = useTranslation();

  return (
    <div className="inline-flex rounded-lg border border-neutral-200/80 dark:border-neutral-700 p-0.5">
      <button
        type="button"
        onClick={() => onChange("text")}
        className={`rounded-md px-2.5 py-1 text-xs transition ${
          mode === "text"
            ? "bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-100"
            : "text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300"
        }`}
      >
        {t("search.modeText")}
      </button>
      <button
        type="button"
        onClick={() => onChange("semantic")}
        className={`rounded-md px-2.5 py-1 text-xs transition ${
          mode === "semantic"
            ? "bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-100"
            : "text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300"
        }`}
      >
        {t("search.modeSemantic")}
      </button>
    </div>
  );
}

export function SearchResultsView() {
  const { t } = useTranslation();
  const { searchQuery, searchMode } = useAppStore();
  const parentRef = useRef<HTMLDivElement>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [relevanceById, setRelevanceById] = useState<Map<number, number>>(new Map());
  const [usedSemantic, setUsedSemantic] = useState<boolean | null>(null);
  const [totalCount, setTotalCount] = useState(0);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);
  const [aiStatus, setAiStatus] = useState<AiStatus | null>(null);

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );
  const hasMore = searchMode === "text" && media.length < totalCount;

  const loadInitial = useCallback(async (query: string, mode: SearchMode) => {
    const trimmed = query.trim();
    if (!trimmed) {
      setMedia([]);
      setTotalCount(0);
      setRelevanceById(new Map());
      setUsedSemantic(null);
      return;
    }

    setLoading(true);
    try {
      if (mode === "semantic") {
        const response = await semanticSearch(trimmed, PAGE_SIZE);
        setMedia(response.results.map(searchResultToMediaItem));
        setRelevanceById(new Map(response.results.map((r) => [r.media_id, r.relevance])));
        setUsedSemantic(response.used_semantic);
        setTotalCount(response.results.length);
      } else {
        const [items, count] = await Promise.all([
          searchMedia(trimmed, PAGE_SIZE, 0),
          searchMediaCount(trimmed),
        ]);
        setMedia(items);
        setRelevanceById(new Map());
        setUsedSemantic(null);
        setTotalCount(count);
      }
    } catch (err) {
      console.error("Failed to search media:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (searchMode !== "semantic") {
      setAiStatus(null);
      return;
    }

    let cancelled = false;
    void getAiStatus()
      .then((status) => {
        if (!cancelled) setAiStatus(status);
      })
      .catch(() => {
        if (!cancelled) setAiStatus(null);
      });

    return () => {
      cancelled = true;
    };
  }, [searchMode]);

  useEffect(() => {
    void loadInitial(searchQuery, searchMode);
  }, [searchQuery, searchMode, loadInitial]);

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
    const trimmed = searchQuery.trim();
    if (loadingMore || !hasMore || !trimmed || searchMode !== "text") return;
    setLoadingMore(true);
    try {
      const items = await searchMedia(trimmed, PAGE_SIZE, media.length);
      setMedia((prev) => [...prev, ...items]);
    } catch (err) {
      console.error("Failed to load more search results:", err);
    } finally {
      setLoadingMore(false);
    }
  }, [hasMore, loadingMore, media.length, searchMode, searchQuery]);

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

  const handleModeChange = (mode: SearchMode) => {
    setSearchMode(mode);
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
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <h2 className="text-sm font-medium text-neutral-700 dark:text-neutral-200">
              {t("search.results")}
            </h2>
            <span
              className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                searchMode === "semantic"
                  ? usedSemantic === false
                    ? "bg-amber-500/15 text-amber-700 dark:text-amber-300"
                    : "bg-violet-500/15 text-violet-700 dark:text-violet-300"
                  : "bg-neutral-500/10 text-neutral-600 dark:text-neutral-400"
              }`}
            >
              {searchMode === "semantic"
                ? usedSemantic === false
                  ? t("search.modeKeywordFallback")
                  : t("search.modeSemantic")
                : t("search.modeText")}
            </span>
          </div>
          <p className="mt-0.5 text-sm text-neutral-500">
            {searchQuery.trim() && (
              <>
                <span className="text-neutral-500 dark:text-neutral-400">
                  &ldquo;{searchQuery.trim()}&rdquo;
                </span>
                {" · "}
              </>
            )}
            {t("search.count", { count: totalCount })}
          </p>
        </div>
        <SearchModeToggle mode={searchMode} onChange={handleModeChange} />
      </div>

      {searchMode === "semantic" && usedSemantic === false && (
        <div className="border-b border-amber-200/80 bg-amber-50 px-4 py-2 text-sm text-amber-900 dark:border-amber-900/50 dark:bg-amber-950/40 dark:text-amber-200">
          <p>{t("search.semanticFallback")}</p>
          <p className="mt-1 text-xs opacity-90">{t("search.semanticDownloadHint")}</p>
        </div>
      )}

      {searchMode === "semantic" && usedSemantic === true && (
        <div className="border-b border-violet-200/80 bg-violet-50 px-4 py-2 text-sm text-violet-900 dark:border-violet-900/50 dark:bg-violet-950/40 dark:text-violet-200">
          <p>{t("search.semanticActive")}</p>
        </div>
      )}

      {searchMode === "semantic" && aiStatus && !aiStatus.clip_available && usedSemantic === null && (
        <div className="border-b border-amber-200/80 bg-amber-50 px-4 py-2 text-sm text-amber-900 dark:border-amber-900/50 dark:bg-amber-950/40 dark:text-amber-200">
          <p>{t("search.semanticFallback")}</p>
          <p className="mt-1 text-xs opacity-90">{t("search.semanticDownloadHint")}</p>
        </div>
      )}

      {media.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-neutral-500">
          <p>{t("search.noResults")}</p>
          <p className="text-sm text-neutral-600">{t("search.noResultsHint")}</p>
        </div>
      ) : (
        <div ref={parentRef} className="flex-1 overflow-y-auto px-1 py-1">
          <div
            className="grid gap-[3px]"
            style={{
              gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
            }}
          >
            {media.map((item) => {
              const relevance = relevanceById.get(item.id);
              return (
                <div key={item.id} className="relative">
                  <PhotoCard
                    item={item}
                    selected={false}
                    selectedMediaIds={[]}
                    onSelect={() => openViewer(item.id)}
                    onOpen={openViewer}
                  />
                  {relevance != null && (
                    <span
                      className={`absolute bottom-1 right-1 rounded px-1.5 py-0.5 text-[10px] tabular-nums text-white ${
                        usedSemantic ? "bg-violet-600/80" : "bg-black/60"
                      }`}
                    >
                      {usedSemantic ? t("search.similarity") : t("search.relevance")}:{" "}
                      {(relevance * 100).toFixed(0)}%
                    </span>
                  )}
                </div>
              );
            })}
          </div>
          {loadingMore && (
            <div className="py-4 text-center text-sm text-neutral-500">{t("gallery.loading")}</div>
          )}
        </div>
      )}
    </div>
  );
}
