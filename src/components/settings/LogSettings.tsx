import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "@/i18n/useTranslation";

interface LogConfig {
  level: string;
  retention_days: number;
  max_size_mb: number;
}

interface LogFileInfo {
  path: string;
  size_bytes: number;
  modified: string;
}

const LOG_LEVELS = ["trace", "debug", "info", "warn", "error"];

export function LogSettings() {
  const { t } = useTranslation();
  const [config, setConfig] = useState<LogConfig>({
    level: "info",
    retention_days: 7,
    max_size_mb: 100,
  });
  const [logDir, setLogDir] = useState("");
  const [logFiles, setLogFiles] = useState<LogFileInfo[]>([]);
  const [saved, setSaved] = useState(false);
  const savedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<LogConfig>("get_log_config").then(setConfig).catch(console.error);
    invoke<string>("get_log_directory").then(setLogDir).catch(console.error);
    invoke<LogFileInfo[]>("get_log_files")
      .then(setLogFiles)
      .catch(console.error);

    return () => {
      if (savedTimeoutRef.current) {
        clearTimeout(savedTimeoutRef.current);
      }
    };
  }, []);

  const handleSave = async () => {
    try {
      await invoke("set_log_config", { config });
      setSaved(true);
      if (savedTimeoutRef.current) {
        clearTimeout(savedTimeoutRef.current);
      }
      savedTimeoutRef.current = setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("Failed to save log config:", e);
    }
  };

  const handleCleanup = async () => {
    try {
      await invoke("cleanup_logs");
      const files = await invoke<LogFileInfo[]>("get_log_files");
      setLogFiles(files);
    } catch (e) {
      console.error("Failed to cleanup logs:", e);
    }
  };

  const totalSizeMB =
    logFiles.reduce((sum, f) => sum + f.size_bytes, 0) / (1024 * 1024);

  return (
    <section className="settings-section px-6 py-5">
      <h2 className="text-base font-semibold text-neutral-900 dark:text-neutral-100">
        {t("settings.logTitle")}
      </h2>
      <p className="mt-1 text-sm text-neutral-500">
        {t("settings.logDescription")}
      </p>

      <div className="mt-4 space-y-4">
        <div className="flex items-center gap-4">
          <label className="w-28 text-sm font-medium text-neutral-700 dark:text-neutral-300">
            {t("settings.logLevel")}
          </label>
          <select
            value={config.level}
            onChange={(e) =>
              setConfig((c) => ({ ...c, level: e.target.value }))
            }
            className="rounded-lg border border-neutral-300 bg-white px-3 py-1.5 text-sm dark:border-neutral-600 dark:bg-neutral-800"
          >
            {LOG_LEVELS.map((lvl) => (
              <option key={lvl} value={lvl}>
                {lvl.toUpperCase()}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-4">
          <label className="w-28 text-sm font-medium text-neutral-700 dark:text-neutral-300">
            {t("settings.logRetention")}
          </label>
          <input
            type="number"
            min={1}
            max={90}
            value={config.retention_days}
            onChange={(e) =>
              setConfig((c) => ({
                ...c,
                retention_days: Math.max(
                  1,
                  Math.min(90, Number(e.target.value) || 7),
                ),
              }))
            }
            className="w-20 rounded-lg border border-neutral-300 bg-white px-3 py-1.5 text-sm dark:border-neutral-600 dark:bg-neutral-800"
          />
          <span className="text-sm text-neutral-500">
            {t("settings.logDays")}
          </span>
        </div>

        <div className="flex items-center gap-4">
          <label className="w-28 text-sm font-medium text-neutral-700 dark:text-neutral-300">
            {t("settings.logMaxSize")}
          </label>
          <input
            type="number"
            min={10}
            max={1024}
            value={config.max_size_mb}
            onChange={(e) =>
              setConfig((c) => ({
                ...c,
                max_size_mb: Math.max(
                  10,
                  Math.min(1024, Number(e.target.value) || 100),
                ),
              }))
            }
            className="w-20 rounded-lg border border-neutral-300 bg-white px-3 py-1.5 text-sm dark:border-neutral-600 dark:bg-neutral-800"
          />
          <span className="text-sm text-neutral-500">MB</span>
        </div>

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => void handleSave()}
            className="rounded-lg bg-blue-600 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-blue-700"
          >
            {saved ? t("settings.logSaved") : t("settings.logSave")}
          </button>
          <button
            type="button"
            onClick={() => void handleCleanup()}
            className="rounded-lg border border-neutral-300 px-4 py-1.5 text-sm font-medium text-neutral-700 transition-colors hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-300 dark:hover:bg-neutral-800"
          >
            {t("settings.logCleanup")}
          </button>
        </div>
      </div>

      <div className="mt-4 rounded-lg bg-neutral-50 p-3 dark:bg-neutral-800/50">
        <div className="flex items-center justify-between">
          <span className="text-xs text-neutral-500">
            {t("settings.logCount")}: {logFiles.length}
          </span>
          <span className="text-xs text-neutral-500">
            {t("settings.logTotalSize")}: {totalSizeMB.toFixed(1)} MB
          </span>
        </div>
        <p className="mt-1 truncate text-xs text-neutral-400">{logDir}</p>
      </div>
    </section>
  );
}
