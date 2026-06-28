import { useEffect, useRef, useState } from "react";
import { Sidebar } from "@/components/layout/Sidebar";
import { MainContent } from "@/components/layout/MainContent";
import { useTranslation } from "@/i18n/useTranslation";
import {
  getMediaCount,
  getMediaList,
  listWatchedFolders,
  onScanProgress,
} from "@/lib/tauri";
import {
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

    let unlisten: (() => void) | undefined;
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
      unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-white text-neutral-900 dark:bg-neutral-950 dark:text-neutral-100">
      <Sidebar />
      <main className="flex flex-1 flex-col overflow-hidden">
        <header className="flex items-center justify-between gap-4 border-b border-neutral-200 px-6 py-3 dark:border-neutral-800">
          <h1 className="shrink-0 text-lg font-semibold">{t("app.title")}</h1>
          <div className="mx-4 flex max-w-md flex-1">
            <input
              type="search"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              placeholder={t("search.placeholder")}
              className="w-full rounded-md border border-neutral-300 bg-neutral-50 px-3 py-1.5 text-sm text-neutral-900 placeholder:text-neutral-400 focus:border-neutral-400 focus:outline-none dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-200 dark:placeholder:text-neutral-500 dark:focus:border-neutral-500"
            />
          </div>
          {totalCount > 0 && !searchQuery.trim() && (
            <span className="shrink-0 text-sm text-neutral-400">
              {t("gallery.count", { count: totalCount })}
            </span>
          )}
          {searchQuery.trim() && <span className="shrink-0 w-0" aria-hidden="true" />}
        </header>
        <MainContent />
      </main>
    </div>
  );
}
