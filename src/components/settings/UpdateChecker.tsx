import { useState } from "react";
import { open } from "@tauri-apps/plugin-shell";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "@/i18n/useTranslation";

const RELEASES_URL = "https://github.com/halft0n/LightFrame/releases";

type CheckState = "idle" | "checking" | "latest" | "available" | "error";

export function UpdateChecker() {
  const { t } = useTranslation();
  const [state, setState] = useState<CheckState>("idle");
  const [latestVersion, setLatestVersion] = useState<string | null>(null);
  const [currentVersion, setCurrentVersion] = useState<string | null>(null);

  const handleCheck = async () => {
    setState("checking");
    try {
      const current = await invoke<string>("get_app_version");
      setCurrentVersion(current);

      const res = await fetch(
        "https://api.github.com/repos/halft0n/LightFrame/releases/latest",
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = (await res.json()) as { tag_name: string };
      const remote = data.tag_name.replace(/^v/, "");
      setLatestVersion(remote);
      setState(remote === current ? "latest" : "available");
    } catch {
      setState("error");
    }
  };

  const handleOpenReleases = async () => {
    try {
      await open(RELEASES_URL);
    } catch {
      window.open(RELEASES_URL, "_blank");
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
            {currentVersion
              ? `v${currentVersion} · ${t("updates.subtitle")}`
              : t("updates.subtitle")}
          </p>
          {state === "latest" && (
            <p className="mt-1 text-sm font-medium text-emerald-600 dark:text-emerald-400">
              {t("updates.upToDate")}
            </p>
          )}
          {state === "available" && latestVersion && (
            <p className="mt-1 text-sm font-medium text-blue-600 dark:text-blue-400">
              v{latestVersion}{" "}
              <button
                type="button"
                onClick={() => void handleOpenReleases()}
                className="underline hover:no-underline"
              >
                {t("updates.check")}
              </button>
            </p>
          )}
          {state === "error" && (
            <p className="mt-1 text-sm text-red-600 dark:text-red-400">
              {t("updates.checkFailed")}
            </p>
          )}
        </div>
        <button
          type="button"
          onClick={() => void handleCheck()}
          disabled={state === "checking"}
          className="rounded-lg border border-neutral-200 px-4 py-2 text-sm font-medium text-neutral-700 transition-colors hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
        >
          {state === "checking" ? t("updates.checking") : t("updates.check")}
        </button>
      </div>
    </section>
  );
}
