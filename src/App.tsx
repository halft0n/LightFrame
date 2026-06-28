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

export default function App() {
  const { t } = useTranslation();
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
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <main className="flex flex-1 flex-col overflow-hidden">
        <header className="flex items-center justify-between gap-4 border-b border-neutral-800 px-6 py-3">
          <h1 className="shrink-0 text-lg font-semibold">{t("app.title")}</h1>
          <div className="mx-4 flex max-w-md flex-1">
            <input
              type="search"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              placeholder={t("search.placeholder")}
              className="w-full rounded-md border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-sm text-neutral-200 placeholder:text-neutral-500 focus:border-neutral-500 focus:outline-none"
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
