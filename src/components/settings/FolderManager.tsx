import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  addWatchedFolder,
  removeWatchedFolder,
  scanFolder,
  type ScanStatus,
} from "@/lib/tauri";
import { addFolder, removeFolder, updateFolder, useAppStore, type Theme } from "@/store/appStore";
import { changeTheme } from "@/hooks/useTheme";
import { useTranslation } from "@/i18n/useTranslation";
import { EmptyState } from "@/components/ui/EmptyState";
import { UpdateChecker } from "@/components/settings/UpdateChecker";

function ScanIndicator({ status }: { status: ScanStatus }) {
  const { t } = useTranslation();

  if (status === "scanning") {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-600 dark:text-blue-400">
        <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-blue-500" />
        {t("settings.scanning")}
      </span>
    );
  }

  if (status === "error") {
    return (
      <span className="inline-flex rounded-full bg-red-500/10 px-2 py-0.5 text-xs font-medium text-red-600 dark:text-red-400">
        {t("settings.error")}
      </span>
    );
  }

  return null;
}

function formatLastScan(value?: string | null): string {
  if (!value) return "—";
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

export function FolderManager() {
  const { t } = useTranslation();
  const { watchedFolders, theme } = useAppStore();
  const [adding, setAdding] = useState(false);
  const [rescanningAll, setRescanningAll] = useState(false);

  const themeOptions: { value: Theme; labelKey: string }[] = [
    { value: "light", labelKey: "theme.light" },
    { value: "dark", labelKey: "theme.dark" },
    { value: "system", labelKey: "theme.system" },
  ];

  const isTauri = Boolean(window.__TAURI_INTERNALS__);

  const handleAddFolder = async () => {
    if (!isTauri) {
      alert(t("settings.tauriOnly"));
      return;
    }

    const selected = await open({
      directory: true,
      multiple: false,
      title: t("settings.addFolder"),
    });

    if (!selected || Array.isArray(selected)) return;

    setAdding(true);
    try {
      const folder = await addWatchedFolder(selected);
      addFolder(folder);
    } catch (err) {
      console.error("Failed to add watched folder:", err);
    } finally {
      setAdding(false);
    }
  };

  const handleRemoveFolder = async (id: number) => {
    try {
      await removeWatchedFolder(id);
      removeFolder(id);
    } catch (err) {
      console.error("Failed to remove watched folder:", err);
    }
  };

  const handleRescanFolder = async (folderId: number) => {
    updateFolder(folderId, { scan_status: "scanning" });
    try {
      await scanFolder(folderId);
    } catch {
      updateFolder(folderId, { scan_status: "error" });
    }
  };

  const handleRescanAll = async () => {
    const targets = watchedFolders.filter((f) => f.scan_status !== "scanning");
    if (targets.length === 0) return;

    setRescanningAll(true);
    try {
      await Promise.all(targets.map((f) => handleRescanFolder(f.id)));
    } finally {
      setRescanningAll(false);
    }
  };

  return (
    <div className="page-enter flex flex-1 flex-col overflow-hidden">
      <section className="settings-section px-6 py-5">
        <h2 className="text-base font-semibold text-neutral-900 dark:text-neutral-100">
          {t("theme.title")}
        </h2>
        <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400">{t("theme.subtitle")}</p>
        <div className="mt-4 flex flex-wrap gap-2">
          {themeOptions.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => changeTheme(opt.value)}
              className={`theme-pill px-5 py-2 text-sm font-medium ${
                theme === opt.value
                  ? "theme-pill-active text-white"
                  : "bg-neutral-100 text-neutral-700 hover:bg-neutral-200 dark:bg-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-700"
              }`}
            >
              {t(opt.labelKey)}
            </button>
          ))}
        </div>
      </section>

      <UpdateChecker />

      <section className="settings-section px-6 py-5">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="text-base font-semibold text-neutral-900 dark:text-neutral-100">
              {t("settings.folders")}
            </h2>
            <p className="mt-1 text-sm text-neutral-500">
              {watchedFolders.length > 0
                ? t("gallery.count", { count: watchedFolders.reduce((n, f) => n + f.media_count, 0) })
                : t("main.addFolder")}
            </p>
          </div>
          <div className="flex items-center gap-2">
            {watchedFolders.length > 0 && (
              <button
                type="button"
                onClick={() => void handleRescanAll()}
                disabled={rescanningAll || watchedFolders.some((f) => f.scan_status === "scanning")}
                className="rounded-lg border border-neutral-200 px-4 py-2 text-sm font-medium text-neutral-700 transition-colors hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
              >
                {rescanningAll ? t("settings.scanning") : t("settings.rescanAll")}
              </button>
            )}
            <button
              type="button"
              onClick={() => void handleAddFolder()}
              disabled={adding}
              className="rounded-lg bg-gradient-to-r from-blue-600 to-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition-all hover:from-blue-500 hover:to-indigo-500 disabled:opacity-50"
            >
              {t("settings.addFolder")}
            </button>
          </div>
        </div>
      </section>

      <div className="flex-1 overflow-y-auto px-6 py-5">
        {watchedFolders.length === 0 ? (
          <EmptyState variant="folder" title={t("main.addFolder")} />
        ) : (
          <ul className="grid gap-3 sm:grid-cols-1 lg:grid-cols-2">
            {watchedFolders.map((folder) => (
              <li key={folder.id} className="settings-card p-4">
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-blue-500/10 text-blue-600 dark:text-blue-400">
                        <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden="true">
                          <path d="M4 6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v14a1 1 0 0 1-1.447.894L12 17.118l-6.553 3.776A1 1 0 0 1 4 20V6z" />
                        </svg>
                      </div>
                      <p className="truncate text-sm font-semibold text-neutral-900 dark:text-neutral-100">
                        {folder.path.split(/[/\\]/).pop() ?? folder.path}
                      </p>
                    </div>
                    <p className="mt-2 truncate pl-11 text-xs text-neutral-500">{folder.path}</p>
                    <div className="mt-3 flex flex-wrap items-center gap-2 pl-11">
                      <span className="rounded-md bg-neutral-100 px-2 py-0.5 text-xs text-neutral-600 dark:bg-neutral-800 dark:text-neutral-400">
                        {t("folder.mediaCount")}: {folder.media_count}
                      </span>
                      <span className="text-xs text-neutral-400">
                        {t("folder.lastScan")}: {formatLastScan(folder.last_scan)}
                      </span>
                      <ScanIndicator status={folder.scan_status} />
                    </div>
                  </div>

                  <div className="flex shrink-0 flex-col gap-1.5">
                    <button
                      type="button"
                      onClick={() => void handleRescanFolder(folder.id)}
                      disabled={folder.scan_status === "scanning"}
                      className="rounded-lg px-3 py-1.5 text-xs font-medium text-neutral-600 transition-colors hover:bg-neutral-200 disabled:opacity-50 dark:text-neutral-400 dark:hover:bg-neutral-800"
                    >
                      {folder.scan_status === "scanning" ? t("settings.scanning") : t("settings.rescan")}
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleRemoveFolder(folder.id)}
                      className="rounded-lg px-3 py-1.5 text-xs font-medium text-red-500 transition-colors hover:bg-red-50 dark:hover:bg-red-950/40"
                    >
                      {t("settings.removeFolder")}
                    </button>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
