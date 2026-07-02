import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  cancelDownload,
  downloadModel,
  getModelStatus,
  openModelsDir,
  type ModelDownloadProgress,
  type ModelFileStatus,
  type ModelStatus,
} from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";
import { localizeError } from "@/lib/errors";

function StatusBadge({ available }: { available: boolean }) {
  const { t } = useTranslation();
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium ${
        available
          ? "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
          : "bg-neutral-500/10 text-neutral-600 dark:text-neutral-400"
      }`}
    >
      {available ? "✅" : "❌"}{" "}
      {available ? t("ai.available") : t("ai.unavailable")}
    </span>
  );
}

function formatFileSize(bytes: number | null): string {
  if (bytes == null) return "—";
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / 1024).toFixed(0)} KB`;
}

function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec >= 1024 * 1024) {
    return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
  }
  return `${(bytesPerSec / 1024).toFixed(0)} KB/s`;
}

interface DownloadProgressState {
  downloaded: number;
  total: number;
  speedBps: number;
}

const CIRCLE_SIZE = 36;
const STROKE_WIDTH = 3;
const RADIUS = (CIRCLE_SIZE - STROKE_WIDTH) / 2;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

function CircularProgress({
  progress,
  onCancel,
}: {
  progress: DownloadProgressState;
  onCancel: () => void;
}) {
  const isConnecting = progress.total === 0;
  const percent = isConnecting
    ? 0
    : Math.min(1, progress.downloaded / progress.total);
  const offset = CIRCUMFERENCE * (1 - percent);

  return (
    <button
      type="button"
      onClick={onCancel}
      className="group relative flex items-center justify-center rounded-full p-1 transition-colors hover:bg-neutral-100 dark:hover:bg-neutral-800"
      title="Cancel download"
    >
      <svg
        width={CIRCLE_SIZE}
        height={CIRCLE_SIZE}
        className={isConnecting ? "animate-spin" : "-rotate-90"}
      >
        <circle
          cx={CIRCLE_SIZE / 2}
          cy={CIRCLE_SIZE / 2}
          r={RADIUS}
          fill="none"
          stroke="currentColor"
          strokeWidth={STROKE_WIDTH}
          className="text-neutral-200 dark:text-neutral-700"
        />
        {isConnecting ? (
          <circle
            cx={CIRCLE_SIZE / 2}
            cy={CIRCLE_SIZE / 2}
            r={RADIUS}
            fill="none"
            stroke="currentColor"
            strokeWidth={STROKE_WIDTH}
            strokeDasharray={`${CIRCUMFERENCE * 0.25} ${CIRCUMFERENCE * 0.75}`}
            strokeLinecap="round"
            className="text-blue-500"
          />
        ) : (
          <circle
            cx={CIRCLE_SIZE / 2}
            cy={CIRCLE_SIZE / 2}
            r={RADIUS}
            fill="none"
            stroke="currentColor"
            strokeWidth={STROKE_WIDTH}
            strokeDasharray={CIRCUMFERENCE}
            strokeDashoffset={offset}
            strokeLinecap="round"
            className="text-blue-500 transition-[stroke-dashoffset] duration-200"
          />
        )}
      </svg>
      <span className="absolute inset-0 flex items-center justify-center">
        <svg
          viewBox="0 0 16 16"
          fill="currentColor"
          className="h-3.5 w-3.5 text-neutral-400 transition-colors group-hover:text-neutral-600 dark:text-neutral-500 dark:group-hover:text-neutral-300"
        >
          <rect x="4" y="4" width="8" height="8" rx="1.5" />
        </svg>
      </span>
    </button>
  );
}

function DownloadInfo({ progress }: { progress: DownloadProgressState }) {
  const { t } = useTranslation();
  const { downloaded, total, speedBps } = progress;
  const isConnecting = total === 0 && downloaded === 0;

  if (isConnecting) {
    return (
      <p className="mt-1 text-[11px] text-neutral-500 dark:text-neutral-400">
        {t("ai.connecting")}
      </p>
    );
  }

  const percent = total > 0 ? Math.min(100, (downloaded / total) * 100) : null;

  return (
    <p className="mt-1 text-[11px] tabular-nums text-neutral-500 dark:text-neutral-400">
      {percent != null ? `${percent.toFixed(1)}%` : "…"}
      {" · "}
      {formatFileSize(downloaded)}
      {total > 0 && <> / {formatFileSize(total)}</>}
      {speedBps > 0 && <> · {formatSpeed(speedBps)}</>}
    </p>
  );
}

function ModelRow({
  model,
  downloading,
  progress,
  onDownload,
  onCancel,
}: {
  model: ModelFileStatus;
  downloading: boolean;
  progress: DownloadProgressState | null;
  onDownload: (filename: string) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const [showInfo, setShowInfo] = useState(false);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const descKey = `ai.model.${model.filename}` as Parameters<typeof t>[0];
  const localizedDesc = t(descKey) !== descKey ? t(descKey) : model.description;

  useEffect(() => {
    return () => {
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
    };
  }, []);

  const handleEnter = () => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    setShowInfo(true);
  };
  const handleLeave = () => {
    hideTimerRef.current = setTimeout(() => setShowInfo(false), 150);
  };

  return (
    <li className="rounded-lg border border-neutral-200/80 px-3 py-3 dark:border-neutral-700">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <p className="text-sm font-medium text-neutral-800 dark:text-neutral-200">
              {model.name}
            </p>
            <span
              className="relative"
              onMouseEnter={handleEnter}
              onMouseLeave={handleLeave}
            >
              <svg viewBox="0 0 16 16" fill="currentColor" className="h-3.5 w-3.5 cursor-help text-neutral-400 dark:text-neutral-500">
                <path fillRule="evenodd" d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm6.5-2.75A1.5 1.5 0 0 1 8 3.75h.01a1.5 1.5 0 0 1 0 3H8A1.5 1.5 0 0 1 6.5 5.25ZM8 7a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 7Z" clipRule="evenodd" />
              </svg>
              {showInfo && (
                <span
                  className="absolute bottom-full left-1/2 z-50 mb-1.5 w-64 -translate-x-1/2 select-text rounded-md bg-neutral-900 px-2.5 py-2 text-[11px] leading-snug text-white shadow-lg dark:bg-neutral-700"
                  onMouseEnter={handleEnter}
                  onMouseLeave={handleLeave}
                >
                  <span className="block">{localizedDesc}</span>
                  <a
                    href={model.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="mt-1.5 block break-all text-blue-300 underline hover:text-blue-200"
                  >
                    {model.url}
                  </a>
                </span>
              )}
            </span>
          </div>
          <p className="mt-1 font-mono text-[11px] text-neutral-500 dark:text-neutral-400">
            {model.filename}
            {model.installed && model.file_size_bytes != null && (
              <> · {formatFileSize(model.file_size_bytes)}</>
            )}
            {!model.installed && <> · ~{model.size_mb} MB</>}
          </p>
          {downloading && progress && <DownloadInfo progress={progress} />}
        </div>
        <div className="flex items-center gap-2">
          <StatusBadge available={model.installed} />
          {!model.installed &&
            (downloading && progress ? (
              <CircularProgress progress={progress} onCancel={onCancel} />
            ) : (
              <button
                type="button"
                onClick={() => onDownload(model.filename)}
                disabled={downloading}
                className="rounded-md bg-blue-600 px-2.5 py-1 text-xs font-medium text-white transition hover:bg-blue-500 disabled:opacity-50"
              >
                {t("ai.downloadModel")}
              </button>
            ))}
        </div>
      </div>
    </li>
  );
}

export function AiSettings() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<ModelStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [opening, setOpening] = useState(false);
  const [downloads, setDownloads] = useState<
    Map<string, DownloadProgressState>
  >(new Map());
  const [downloadErrors, setDownloadErrors] = useState<Map<string, string>>(
    new Map(),
  );
  const [statusError, setStatusError] = useState<string | null>(null);
  const speedSamplesRef = useRef<Map<string, { downloaded: number; at: number }>>(
    new Map(),
  );
  const mountedRef = useRef(true);
  const progressBufferRef = useRef(new Map<string, DownloadProgressState>());
  const cancelledRef = useRef(new Set<string>());

  const loadStatus = useCallback(async () => {
    setLoading(true);
    setStatusError(null);
    try {
      const next = await getModelStatus();
      setStatus(next);
    } catch (err) {
      setStatus(null);
      setStatusError(localizeError(err, t));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    let unlisten: UnlistenFn | undefined;

    const flushTimer = setInterval(() => {
      if (!mountedRef.current) return;
      const buf = progressBufferRef.current;
      if (buf.size === 0) return;
      const snapshot = new Map(buf);
      buf.clear();
      setDownloads((prev) => {
        const next = new Map(prev);
        for (const [k, v] of snapshot) next.set(k, v);
        return next;
      });
    }, 200);

    let staleCheckInFlight = false;
    const staleTimer = setInterval(async () => {
      if (!mountedRef.current || staleCheckInFlight) return;
      const now = Date.now();
      const stale: string[] = [];
      for (const [filename, sample] of speedSamplesRef.current) {
        if (now - sample.at > 5000) stale.push(filename);
      }
      if (stale.length === 0) return;
      staleCheckInFlight = true;
      try {
        const fresh = await getModelStatus();
        if (!mountedRef.current) return;
        setStatus(fresh);
        setDownloads((prev) => {
          let next = prev;
          for (const fn of stale) {
            const m = fresh.models.find((x) => x.filename === fn);
            if (
              m?.installed ||
              now - (speedSamplesRef.current.get(fn)?.at ?? 0) > 90000
            ) {
              if (next === prev) next = new Map(prev);
              next.delete(fn);
              speedSamplesRef.current.delete(fn);
            }
          }
          return next;
        });
        setDownloadErrors((prev) => {
          let next = prev;
          for (const fn of stale) {
            if (prev.has(fn)) {
              if (next === prev) next = new Map(prev);
              next.delete(fn);
            }
          }
          return next;
        });
      } catch {
        // ignore status check errors during reconciliation
      } finally {
        staleCheckInFlight = false;
      }
    }, 5000);

    void listen<ModelDownloadProgress>("model-download-progress", (event) => {
      const { filename, downloaded, total } = event.payload;
      if (!mountedRef.current || cancelledRef.current.has(filename)) return;

      const now = Date.now();
      const prev = speedSamplesRef.current.get(filename);
      let speedBps = 0;
      if (prev && now > prev.at) {
        const deltaBytes = downloaded - prev.downloaded;
        const deltaSec = (now - prev.at) / 1000;
        if (deltaSec > 0 && deltaBytes >= 0) {
          speedBps = deltaBytes / deltaSec;
        }
      }
      speedSamplesRef.current.set(filename, { downloaded, at: now });
      progressBufferRef.current.set(filename, { downloaded, total, speedBps });
    }).then((fn) => {
      if (mountedRef.current) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    return () => {
      clearInterval(flushTimer);
      clearInterval(staleTimer);
      unlisten?.();
    };
  }, []);

  const handleOpenModelsDir = async () => {
    if (!window.__TAURI_INTERNALS__) {
      alert(t("settings.tauriOnly"));
      return;
    }

    setOpening(true);
    try {
      await openModelsDir();
    } catch (err) {
      console.error("Failed to open models directory:", err);
    } finally {
      setOpening(false);
    }
  };

  const handleDownload = async (filename: string) => {
    if (!window.__TAURI_INTERNALS__) {
      alert(t("settings.tauriOnly"));
      return;
    }

    cancelledRef.current.delete(filename);
    setDownloads((prev) => {
      const next = new Map(prev);
      next.set(filename, { downloaded: 0, total: 0, speedBps: 0 });
      return next;
    });
    setDownloadErrors((prev) => {
      const next = new Map(prev);
      next.delete(filename);
      return next;
    });
    speedSamplesRef.current.delete(filename);

    try {
      await downloadModel(filename);
      if (mountedRef.current) await loadStatus();
    } catch (err) {
      const raw = err instanceof Error ? err.message : String(err);
      if (!raw.includes("cancelled") && mountedRef.current) {
        setDownloadErrors((prev) => {
          const next = new Map(prev);
          next.set(filename, localizeError(err, t));
          return next;
        });
      }
    } finally {
      if (mountedRef.current) {
        setDownloads((prev) => {
          const next = new Map(prev);
          next.delete(filename);
          return next;
        });
      }
      speedSamplesRef.current.delete(filename);
      progressBufferRef.current.delete(filename);
    }
  };

  const handleCancel = async (filename: string) => {
    cancelledRef.current.add(filename);
    setDownloads((prev) => {
      const next = new Map(prev);
      next.delete(filename);
      return next;
    });
    speedSamplesRef.current.delete(filename);
    progressBufferRef.current.delete(filename);
    try {
      await cancelDownload(filename);
    } catch {
      // best-effort
    }
  };

  return (
    <section className="settings-section px-6 py-5">
      <h2 className="text-base font-semibold text-neutral-900 dark:text-neutral-100">
        {t("ai.settingsTitle")}
      </h2>
      <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400">
        {t("ai.settingsSubtitle")}
      </p>

      {loading ? (
        <p className="mt-4 text-sm text-neutral-500">{t("gallery.loading")}</p>
      ) : status ? (
        <div className="mt-4 space-y-4">
          <ul className="space-y-2">
            {status.models.map((model) => (
              <ModelRow
                key={model.filename}
                model={model}
                downloading={downloads.has(model.filename)}
                progress={downloads.get(model.filename) ?? null}
                onDownload={(name) => void handleDownload(name)}
                onCancel={() => void handleCancel(model.filename)}
              />
            ))}
          </ul>

          {[...downloadErrors.entries()].map(([filename, error]) => (
            <p
              key={filename}
              className="text-sm text-red-600 dark:text-red-400"
            >
              {filename}: {error}
            </p>
          ))}

          <div className="rounded-lg bg-neutral-100 px-3 py-2 dark:bg-neutral-800/80">
            <p className="text-xs font-medium text-neutral-500 dark:text-neutral-400">
              {t("ai.modelsDir")}
            </p>
            <p className="mt-1 break-all font-mono text-xs text-neutral-700 dark:text-neutral-300">
              {status.models_dir}
            </p>
          </div>

          <p className="text-sm text-neutral-600 dark:text-neutral-400">
            {t("ai.manualInstallHint")}
          </p>

          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => void handleOpenModelsDir()}
              disabled={opening}
              className="rounded-lg bg-gradient-to-r from-blue-600 to-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition-all hover:from-blue-500 hover:to-indigo-500 disabled:opacity-50"
            >
              {opening ? t("gallery.loading") : t("ai.openModelsDir")}
            </button>
            <button
              type="button"
              onClick={() => void loadStatus()}
              className="rounded-lg px-4 py-2 text-sm font-medium text-neutral-600 transition-colors hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
            >
              {t("ai.refreshStatus")}
            </button>
          </div>
        </div>
      ) : (
        <div className="mt-4 space-y-3">
          {statusError && (
            <p className="text-sm text-red-600 dark:text-red-400">
              {statusError}
            </p>
          )}
          {!statusError && (
            <p className="text-sm text-neutral-500">
              {t("ai.statusUnavailable")}
            </p>
          )}
        </div>
      )}
    </section>
  );
}
