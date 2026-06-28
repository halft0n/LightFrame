import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  addWatchedFolder,
  removeWatchedFolder,
  scanFolder,
  type ScanStatus,
} from "@/lib/tauri";
import { addFolder, removeFolder, useAppStore } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

function ScanIndicator({ status }: { status: ScanStatus }) {
  const { t } = useTranslation();

  if (status === "scanning") {
    return (
      <span className="inline-flex items-center gap-1.5 text-xs text-blue-400">
        <span className="h-2 w-2 animate-pulse rounded-full bg-blue-400" />
        {t("settings.scanning")}
      </span>
    );
  }

  if (status === "error") {
    return <span className="text-xs text-red-400">Error</span>;
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
  const { watchedFolders } = useAppStore();
  const [adding, setAdding] = useState(false);

  const handleAddFolder = async () => {
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
      await scanFolder(folder.id);
    } finally {
      setAdding(false);
    }
  };

  const handleRemoveFolder = async (id: number) => {
    await removeWatchedFolder(id);
    removeFolder(id);
  };

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-neutral-800 px-6 py-4">
        <h2 className="text-lg font-semibold text-neutral-100">{t("settings.folders")}</h2>
        <button
          type="button"
          onClick={() => void handleAddFolder()}
          disabled={adding}
          className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:opacity-50"
        >
          {t("settings.addFolder")}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-6 py-4">
        {watchedFolders.length === 0 ? (
          <p className="text-sm text-neutral-500">{t("main.addFolder")}</p>
        ) : (
          <ul className="space-y-3">
            {watchedFolders.map((folder) => (
              <li
                key={folder.id}
                className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-4"
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium text-neutral-100">
                      {folder.path.split(/[/\\]/).pop() ?? folder.path}
                    </p>
                    <p className="mt-1 truncate text-xs text-neutral-500">
                      {t("folder.path")}: {folder.path}
                    </p>
                    <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-neutral-400">
                      <span>
                        {t("folder.mediaCount")}: {folder.media_count}
                      </span>
                      <span>
                        {t("folder.lastScan")}: {formatLastScan(folder.last_scan)}
                      </span>
                      <ScanIndicator status={folder.scan_status} />
                    </div>
                  </div>

                  <div className="flex shrink-0 items-center gap-2">
                    <button
                      type="button"
                      onClick={() => void handleRemoveFolder(folder.id)}
                      className="rounded-md px-3 py-1.5 text-xs text-red-400 transition-colors hover:bg-red-950/50 hover:text-red-300"
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
