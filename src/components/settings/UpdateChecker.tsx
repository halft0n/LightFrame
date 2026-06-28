import { useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useTranslation } from "@/i18n/useTranslation";

type UpdateStatus = "idle" | "checking" | "downloading" | "uptodate" | "error";

export function UpdateChecker() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [message, setMessage] = useState<string | null>(null);
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);

  const isTauri = Boolean(window.__TAURI_INTERNALS__);

  const handleCheckForUpdates = async () => {
    if (!isTauri) {
      setMessage(t("settings.tauriOnly"));
      return;
    }

    setStatus("checking");
    setMessage(null);
    setAvailableVersion(null);

    try {
      const update = await check();
      if (update) {
        setAvailableVersion(update.version);
        setStatus("downloading");
        setMessage(t("updates.downloading", { version: update.version }));
        await update.downloadAndInstall();
        await relaunch();
      } else {
        setStatus("uptodate");
        setMessage(t("updates.upToDate"));
      }
    } catch (err) {
      setStatus("error");
      setMessage(err instanceof Error ? err.message : t("updates.checkFailed"));
    }
  };

  return (
    <section className="settings-section px-6 py-5">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-base font-semibold text-neutral-900 dark:text-neutral-100">
            {t("updates.title")}
          </h2>
          <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400">
            {t("updates.subtitle")}
          </p>
          {message && (
            <p
              className={`mt-2 text-sm ${
                status === "error"
                  ? "text-red-600 dark:text-red-400"
                  : status === "uptodate"
                    ? "text-green-600 dark:text-green-400"
                    : "text-neutral-600 dark:text-neutral-400"
              }`}
            >
              {message}
              {availableVersion && status === "downloading" ? ` (${availableVersion})` : ""}
            </p>
          )}
        </div>
        <button
          type="button"
          onClick={() => void handleCheckForUpdates()}
          disabled={status === "checking" || status === "downloading"}
          className="rounded-lg border border-neutral-200 px-4 py-2 text-sm font-medium text-neutral-700 transition-colors hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
        >
          {status === "checking"
            ? t("updates.checking")
            : status === "downloading"
              ? t("updates.downloading", { version: availableVersion ?? "" })
              : t("updates.check")}
        </button>
      </div>
    </section>
  );
}
