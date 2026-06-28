import { useCallback, useEffect, useState } from "react";
import {
  getModelStatus,
  openModelsDir,
  type ModelStatus,
} from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";

const MODELS_RELEASE_URL = "https://github.com/halft0n/CatchLight/releases";

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

export function AiSettings() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<ModelStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [opening, setOpening] = useState(false);

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
          <ul className="space-y-2 text-sm">
            <li className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-neutral-700 dark:text-neutral-300">
                {t("ai.clipModel")}
              </span>
              <StatusBadge available={status.clip_available} />
            </li>
            <li className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-neutral-700 dark:text-neutral-300">
                {t("ai.faceModel")}
              </span>
              <StatusBadge available={status.face_available} />
            </li>
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
            <a
              href={MODELS_RELEASE_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="rounded-lg border border-neutral-200 px-4 py-2 text-sm font-medium text-neutral-700 transition-colors hover:bg-neutral-100 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
            >
              {t("ai.downloadModels")}
            </a>
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
