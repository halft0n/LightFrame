import { useState } from "react";
import { open } from "@tauri-apps/plugin-shell";
import { checkForUpdates, type UpdateCheckResult } from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";

const RELEASES_URL = "https://github.com/halft0n/LightFrame/releases";

type CheckState = "idle" | "checking" | "latest" | "available" | "error";

export function UpdateChecker() {
  const { t } = useTranslation();
  const [state, setState] = useState<CheckState>("idle");
  const [result, setResult] = useState<UpdateCheckResult | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const handleCheck = async () => {
    setState("checking");
    setErrorMsg(null);
    try {
      const r = await checkForUpdates();
      setResult(r);
      setState(r.update_available ? "available" : "latest");
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setState("error");
    }
  };

  const handleOpenReleases = async () => {
    const url = result?.release_url ?? RELEASES_URL;
    try {
      await open(url);
    } catch {
      window.open(url, "_blank");
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
            {result?.current_version
              ? `v${result.current_version} · ${t("updates.subtitle")}`
              : t("updates.subtitle")}
          </p>
          {state === "latest" && (
            <p className="mt-1 text-sm font-medium text-emerald-600 dark:text-emerald-400">
              {t("updates.upToDate")}
            </p>
          )}
          {state === "available" && result && (
            <p className="mt-1 text-sm font-medium text-blue-600 dark:text-blue-400">
              v{result.latest_version}{" "}
              <button
                type="button"
                onClick={() => void handleOpenReleases()}
                className="underline hover:no-underline"
              >
                {t("updates.viewRelease")}
              </button>
            </p>
          )}
          {state === "error" && (
            <p className="mt-1 text-sm text-red-600 dark:text-red-400">
              {errorMsg ?? t("updates.checkFailed")}
            </p>
          )}
        </div>
        <button
          type="button"
          onClick={() => void handleCheck()}
          disabled={state === "checking"}
          className="rounded-lg border border-neutral-200 px-4 py-2 text-sm font-medium text-neutral-700 transition-colors hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
        >
          {state === "checking" ? (
            <span className="flex items-center gap-2">
              <svg
                className="h-4 w-4 animate-spin"
                viewBox="0 0 24 24"
                fill="none"
              >
                <circle
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="3"
                  className="opacity-25"
                />
                <path
                  d="M4 12a8 8 0 018-8"
                  stroke="currentColor"
                  strokeWidth="3"
                  strokeLinecap="round"
                />
              </svg>
              {t("updates.checking")}
            </span>
          ) : (
            t("updates.check")
          )}
        </button>
      </div>
    </section>
  );
}
