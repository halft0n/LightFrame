import { useCallback, useEffect, useState } from "react";
import {
  downloadModel,
  getModelStatus,
  openModelsDir,
  type ModelFileStatus,
  type ModelStatus,
} from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";

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

function ModelRow({
  model,
  downloading,
  onDownload,
}: {
  model: ModelFileStatus;
  downloading: boolean;
  onDownload: (filename: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <li className="rounded-lg border border-neutral-200/80 px-3 py-3 dark:border-neutral-700">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium text-neutral-800 dark:text-neutral-200">
            {model.name}
          </p>
          <p className="mt-0.5 text-xs text-neutral-500 dark:text-neutral-400">
            {model.description}
          </p>
          <p className="mt-1 font-mono text-[11px] text-neutral-500 dark:text-neutral-400">
            {model.filename}
            {model.installed && model.file_size_bytes != null && (
              <> · {formatFileSize(model.file_size_bytes)}</>
            )}
            {!model.installed && <> · ~{model.size_mb} MB</>}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <StatusBadge available={model.installed} />
          {!model.installed && (
            <button
              type="button"
              onClick={() => onDownload(model.filename)}
              disabled={downloading}
              className="rounded-md bg-blue-600 px-2.5 py-1 text-xs font-medium text-white transition hover:bg-blue-500 disabled:opacity-50"
            >
              {downloading ? t("ai.downloading") : t("ai.downloadModel")}
            </button>
          )}
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
  const [downloadingFile, setDownloadingFile] = useState<string | null>(null);

  const loadStatus = useCallback(async () => {
    setLoading(true);
    try {
      const next = await getModelStatus();
      setStatus(next);
    } catch {
      setStatus(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

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

    setDownloadingFile(filename);
    try {
      await downloadModel(filename);
      await loadStatus();
    } catch (err) {
      console.error("Failed to download model:", err);
    } finally {
      setDownloadingFile(null);
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
                downloading={downloadingFile === model.filename}
                onDownload={(filename) => void handleDownload(filename)}
              />
            ))}
          </ul>

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
        <p className="mt-4 text-sm text-neutral-500">{t("ai.statusUnavailable")}</p>
      )}
    </section>
  );
}
