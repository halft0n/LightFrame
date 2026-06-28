import { useEffect, useRef, useState } from "react";
import { Sidebar } from "@/components/layout/Sidebar";
import { MainContent } from "@/components/layout/MainContent";
import { useTranslation } from "@/i18n/useTranslation";
import {
  getMediaCount,
  getMediaList,
  listWatchedFolders,
  onFolderChanged,
  onScanProgress,
  scanFolder,
} from "@/lib/tauri";
import {
  getSnapshot,
  setMedia,
  setScanning,
  setSearchQuery,
  setWatchedFolders,
  updateFolder,
  useAppStore,
} from "@/store/appStore";
import { useTheme } from "@/hooks/useTheme";

export default function App() {
  const { t } = useTranslation();
  useTheme();
  const { totalCount, searchQuery } = useAppStore();
  const [inputValue, setInputValue] = useState(searchQuery);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastAutoRescanRef = useRef(0);

  const RESCAN_COOLDOWN_MS = 30_000;

  useEffect(() => {
    setInputValue(searchQuery);
  }, [searchQuery]);

  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      setSearchQuery(inputValue);
    }, 300);
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [inputValue]);

  useEffect(() => {
    let cancelled = false;

    async function init() {
      try {
        const folders = await listWatchedFolders();
        if (cancelled) return;
        setWatchedFolders(folders);

        if (folders.length > 0) {
          const [items, count] = await Promise.all([
            getMediaList(0, 60),
            getMediaCount(),
          ]);
          if (cancelled) return;
          setMedia(items, count);
        }
      } catch {
        // Backend commands may be unavailable during web-only dev
      }
    }

    void init();

    let unlistenProgress: (() => void) | undefined;
    let unlistenFolder: (() => void) | undefined;

    void onScanProgress((progress) => {
      setScanning(progress.status === "scanning", progress);
      updateFolder(progress.folder_id, {
        scan_status: progress.status,
      });

      if (progress.status === "complete") {
        void (async () => {
          try {
            const [items, count, folders] = await Promise.all([
              getMediaList(0, 60),
              getMediaCount(),
              listWatchedFolders(),
            ]);
            setMedia(items, count);
            setWatchedFolders(folders);
          } catch {
            // ignore refresh errors
          } finally {
            setScanning(false, null);
          }
        })();
      }
    }).then((fn) => {
      unlistenProgress = fn;
    });

    void onFolderChanged((folderId) => {
      updateFolder(folderId, { scan_status: "scanning" });
      void scanFolder(folderId).catch(() => {
        updateFolder(folderId, { scan_status: "error" });
      });
    }).then((fn) => {
      unlistenFolder = fn;
    });

    return () => {
      cancelled = true;
      unlistenProgress?.();
      unlistenFolder?.();
    };
  }, []);

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState !== "visible") return;

      const now = Date.now();
      if (now - lastAutoRescanRef.current < RESCAN_COOLDOWN_MS) return;

      const { watchedFolders } = getSnapshot();
      if (watchedFolders.length === 0) return;

      const foldersToRescan = watchedFolders.filter((folder) => {
        if (folder.scan_status === "scanning") return false;
        if (!folder.last_scan) return true;
        const lastScan = new Date(folder.last_scan).getTime();
        return now - lastScan > RESCAN_COOLDOWN_MS;
      });

      if (foldersToRescan.length === 0) return;

      lastAutoRescanRef.current = now;

      void Promise.all(
        foldersToRescan.map(async (folder) => {
          updateFolder(folder.id, { scan_status: "scanning" });
          try {
            await scanFolder(folder.id);
          } catch {
            updateFolder(folder.id, { scan_status: "error" });
          }
        }),
      );
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => document.removeEventListener("visibilitychange", handleVisibilityChange);
  }, []);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-neutral-50 text-neutral-900 dark:bg-neutral-950 dark:text-neutral-100">
      <Sidebar />
      <main className="flex flex-1 flex-col overflow-hidden">
        <header className="header-glass sticky top-0 z-10 flex h-[44px] shrink-0 items-center gap-3 border-b border-neutral-200/70 px-4 dark:border-neutral-800/70">
          <div className="relative flex flex-1 max-w-2xl">
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
              type="search"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              placeholder={t("search.placeholder")}
              className="search-input w-full rounded-lg py-2 pl-9 pr-3 text-sm text-neutral-900 placeholder:text-neutral-400 dark:text-neutral-200 dark:placeholder:text-neutral-500"
            />
          </div>
          {totalCount > 0 && !searchQuery.trim() && (
            <span className="shrink-0 text-[11px] tabular-nums text-neutral-400 dark:text-neutral-500">
              {totalCount.toLocaleString()}
            </span>
          )}
        </header>
        <div className="main-content-enter flex flex-1 flex-col overflow-hidden">
          <MainContent />
        </div>
      </main>
    </div>
  );
}
