import { useCallback, useEffect, useRef, useState } from "react";
import { Sidebar } from "@/components/layout/Sidebar";
import { MainContent } from "@/components/layout/MainContent";
import { useTranslation } from "@/i18n/useTranslation";
import {
  listWatchedFolders,
  onFolderChanged,
  onScanProgress,
  scanFolder,
} from "@/lib/tauri";
import { localizeError } from "@/lib/errors";
import {
  addSearchHistory,
  clearSearchHistory,
  getSnapshot,
  loadMedia,
  setScanning,
  setSearchQuery,
  setSearchMode,
  setWatchedFolders,
  updateFolder,
  useAppStore,
} from "@/store/appStore";
import { useTheme } from "@/hooks/useTheme";

export default function App() {
  const { t } = useTranslation();
  useTheme();
  const { totalCount, searchQuery, searchHistory, searchMode } = useAppStore();
  const [inputValue, setInputValue] = useState(searchQuery);
  const [searchFocused, setSearchFocused] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastRescanRef = useRef(0);
  const searchContainerRef = useRef<HTMLDivElement>(null);
  const lastHistoryQueryRef = useRef("");

  const RESCAN_COOLDOWN_MS = 5 * 60 * 1000;

  useEffect(() => {
    setInputValue(searchQuery);
  }, [searchQuery]);

  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      setSearchQuery(inputValue);
      const trimmed = inputValue.trim();
      if (trimmed && trimmed !== lastHistoryQueryRef.current) {
        addSearchHistory(trimmed);
        lastHistoryQueryRef.current = trimmed;
      }
      if (!trimmed) {
        lastHistoryQueryRef.current = "";
      }
    }, 300);
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [inputValue]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        searchContainerRef.current &&
        !searchContainerRef.current.contains(event.target as Node)
      ) {
        setSearchFocused(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleHistorySelect = (query: string) => {
    setInputValue(query);
    setSearchQuery(query);
    lastHistoryQueryRef.current = query;
    addSearchHistory(query);
    setSearchFocused(false);
  };

  useEffect(() => {
    let mounted = true;
    let cancelled = false;

    async function init() {
      try {
        const folders = await listWatchedFolders();
        if (cancelled) return;
        setWatchedFolders(folders);

        if (folders.length > 0) {
          await loadMedia();
        }
      } catch {
        // Backend commands may be unavailable during web-only dev
      }
    }

    void init();

    let unlistenProgress: (() => void) | undefined;
    let unlistenFolder: (() => void) | undefined;
    let incrementalRefreshTimer: ReturnType<typeof setTimeout> | undefined;
    let lastIncrementalScanned = 0;

    void onScanProgress((progress) => {
      setScanning(progress.status === "scanning", progress);
      updateFolder(progress.folder_id, {
        scan_status: progress.status,
      });

      if (progress.status === "error") {
        console.error(
          `Scan failed for folder ${progress.folder_id}:`,
          localizeError(new Error(progress.status), t),
        );
      }

      if (progress.status === "scanning" && progress.scanned > lastIncrementalScanned + 9) {
        lastIncrementalScanned = progress.scanned;
        if (!incrementalRefreshTimer) {
          incrementalRefreshTimer = setTimeout(() => {
            incrementalRefreshTimer = undefined;
            void loadMedia().catch(() => {});
          }, 2000);
        }
      }

      if (progress.status === "complete") {
        lastIncrementalScanned = 0;
        if (incrementalRefreshTimer) {
          clearTimeout(incrementalRefreshTimer);
          incrementalRefreshTimer = undefined;
        }
        void (async () => {
          try {
            const folders = await listWatchedFolders();
            await loadMedia();
            setWatchedFolders(folders);
          } catch {
            // ignore refresh errors
          } finally {
            setScanning(false, null);
          }
        })();
      }
    }).then((fn) => {
      if (mounted) {
        unlistenProgress = fn;
      } else {
        fn();
      }
    });

    void onFolderChanged((folderId) => {
      updateFolder(folderId, { scan_status: "scanning" });
      void scanFolder(folderId).catch((err) => {
        console.error(`Failed to scan folder ${folderId}:`, err);
        console.error(localizeError(err, t));
        updateFolder(folderId, { scan_status: "error" });
      });
    }).then((fn) => {
      if (mounted) {
        unlistenFolder = fn;
      } else {
        fn();
      }
    });

    return () => {
      mounted = false;
      cancelled = true;
      unlistenProgress?.();
      unlistenFolder?.();
      if (incrementalRefreshTimer) clearTimeout(incrementalRefreshTimer);
    };
  }, [t]);

  const handleVisibilityChange = useCallback(() => {
    if (document.hidden) return;

    const now = Date.now();
    if (now - lastRescanRef.current < RESCAN_COOLDOWN_MS) return;

    const { watchedFolders } = getSnapshot();
    if (watchedFolders.length === 0) return;

    const foldersToRescan = watchedFolders.filter((folder) => {
      if (folder.scan_status === "scanning") return false;
      if (!folder.last_scan) return true;
      const lastScan = new Date(folder.last_scan).getTime();
      return now - lastScan > RESCAN_COOLDOWN_MS;
    });

    if (foldersToRescan.length === 0) return;

    lastRescanRef.current = now;

    void Promise.all(
      foldersToRescan.map(async (folder) => {
        updateFolder(folder.id, { scan_status: "scanning" });
        try {
          await scanFolder(folder.id);
        } catch (err) {
          console.error(`Failed to rescan folder ${folder.id}:`, err);
          console.error(localizeError(err, t));
          updateFolder(folder.id, { scan_status: "error" });
        }
      }),
    );
  }, [t]);

  useEffect(() => {
    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () =>
      document.removeEventListener("visibilitychange", handleVisibilityChange);
  }, [handleVisibilityChange]);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-neutral-50 text-neutral-900 dark:bg-neutral-950 dark:text-neutral-100">
      <Sidebar />

      {sidebarOpen && (
        <>
          <button
            type="button"
            className="sidebar-overlay-backdrop fixed inset-0 z-40 bg-black/40 md:hidden"
            aria-label={t("sidebar.closeSidebar")}
            onClick={() => setSidebarOpen(false)}
          />
          <Sidebar
            isMobile
            mobileOpen={sidebarOpen}
            onMobileClose={() => setSidebarOpen(false)}
          />
        </>
      )}

      <main className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <header className="header-glass sticky top-0 z-10 flex h-[44px] shrink-0 items-center gap-3 border-b border-neutral-200/70 px-4 dark:border-neutral-800/70">
          <button
            type="button"
            className="rounded-lg p-1.5 text-neutral-600 transition hover:bg-neutral-200/60 md:hidden dark:text-neutral-300 dark:hover:bg-neutral-800"
            aria-label={t("sidebar.openMenu")}
            aria-expanded={sidebarOpen}
            onClick={() => setSidebarOpen((open) => !open)}
          >
            <svg
              viewBox="0 0 24 24"
              className="h-5 w-5"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M4 7h16M4 12h16M4 17h16" strokeLinecap="round" />
            </svg>
          </button>
          <div
            ref={searchContainerRef}
            className="relative flex flex-1 max-w-2xl items-center gap-2"
            role="search"
          >
            <div className="relative flex-1">
              <label htmlFor="app-search" className="sr-only">
                {t("a11y.searchInput")}
              </label>
              <svg
                className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-neutral-400"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <circle cx="11" cy="11" r="7" />
                <path d="M20 20l-3-3" strokeLinecap="round" />
              </svg>
              <input
                id="app-search"
                type="search"
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                onFocus={() => setSearchFocused(true)}
                placeholder={t("search.placeholder")}
                aria-label={t("a11y.searchInput")}
                aria-expanded={searchFocused && searchHistory.length > 0}
                aria-controls={
                  searchFocused && searchHistory.length > 0
                    ? "search-history-list"
                    : undefined
                }
                className="search-input w-full rounded-lg py-2 pl-9 pr-24 text-sm text-neutral-900 placeholder:text-neutral-400 dark:text-neutral-200 dark:placeholder:text-neutral-500 sm:pr-3"
              />
              <span
                className={`pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 rounded-full px-2 py-0.5 text-[10px] font-medium sm:hidden ${
                  searchMode === "semantic"
                    ? "bg-violet-500/15 text-violet-700 dark:text-violet-300"
                    : "bg-neutral-500/10 text-neutral-600 dark:text-neutral-400"
                }`}
              >
                {searchMode === "semantic"
                  ? t("search.modeSemanticShort")
                  : t("search.modeTextShort")}
              </span>
              {searchFocused && searchHistory.length > 0 && (
                <div className="absolute left-0 right-0 top-full z-20 mt-1 overflow-hidden rounded-lg border border-neutral-200 bg-white shadow-lg dark:border-neutral-700 dark:bg-neutral-900">
                  <div className="flex items-center justify-between border-b border-neutral-200 px-3 py-2 dark:border-neutral-700">
                    <span className="text-xs font-medium text-neutral-500">
                      {t("search.recent")}
                    </span>
                    <button
                      type="button"
                      onClick={() => clearSearchHistory()}
                      className="text-xs text-neutral-400 transition hover:text-neutral-600 active:text-neutral-700 dark:hover:text-neutral-300 dark:active:text-neutral-200"
                      aria-label={t("search.clearHistory")}
                    >
                      {t("search.clearHistory")}
                    </button>
                  </div>
                  <ul
                    id="search-history-list"
                    role="listbox"
                    aria-label={t("search.recent")}
                  >
                    {searchHistory.map((query) => (
                      <li key={query} role="option">
                        <button
                          type="button"
                          onClick={() => handleHistorySelect(query)}
                          className="block w-full truncate px-3 py-2 text-left text-sm text-neutral-700 transition hover:bg-neutral-100 active:bg-neutral-200 dark:text-neutral-300 dark:hover:bg-neutral-800 dark:active:bg-neutral-700"
                        >
                          {query}
                        </button>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
            <button
              type="button"
              onClick={() =>
                setSearchMode(searchMode === "text" ? "semantic" : "text")
              }
              className={`shrink-0 rounded-lg border p-2 transition active:scale-95 ${
                searchMode === "semantic"
                  ? "border-violet-300 bg-violet-500/10 text-violet-700 dark:border-violet-700 dark:text-violet-300"
                  : "border-neutral-200/80 text-neutral-500 hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
              }`}
              aria-label={
                searchMode === "semantic"
                  ? t("search.modeSemantic")
                  : t("search.modeText")
              }
              title={
                searchMode === "semantic"
                  ? t("search.modeSemantic")
                  : t("search.modeText")
              }
            >
              {searchMode === "semantic" ? (
                <svg
                  viewBox="0 0 24 24"
                  className="h-4 w-4"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.75"
                  aria-hidden="true"
                >
                  <path
                    d="M12 3l1.9 5.8H20l-4.9 3.6 1.9 5.8L12 14.6 7 18.2l1.9-5.8L4 8.8h6.1L12 3z"
                    strokeLinejoin="round"
                  />
                </svg>
              ) : (
                <svg
                  viewBox="0 0 24 24"
                  className="h-4 w-4"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.75"
                  aria-hidden="true"
                >
                  <path d="M4 7h16M7 12h10M10 17h4" strokeLinecap="round" />
                </svg>
              )}
            </button>
            <div className="hidden shrink-0 sm:flex rounded-lg border border-neutral-200/80 dark:border-neutral-700 p-0.5">
              <button
                type="button"
                onClick={() => setSearchMode("text")}
                className={`rounded-md px-2 py-1 text-[11px] transition ${
                  searchMode === "text"
                    ? "bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-100"
                    : "text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300"
                }`}
              >
                {t("search.modeText")}
              </button>
              <button
                type="button"
                onClick={() => setSearchMode("semantic")}
                className={`rounded-md px-2 py-1 text-[11px] transition ${
                  searchMode === "semantic"
                    ? "bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-100"
                    : "text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300"
                }`}
              >
                {t("search.modeSemantic")}
              </button>
            </div>
          </div>
          {totalCount > 0 && !searchQuery.trim() && (
            <span className="shrink-0 text-[11px] tabular-nums text-neutral-400 dark:text-neutral-500">
              {totalCount.toLocaleString()}
            </span>
          )}
        </header>
        <div className="main-content-enter flex min-h-0 flex-1 flex-col overflow-hidden min-w-0">
          <MainContent />
        </div>
      </main>
    </div>
  );
}
