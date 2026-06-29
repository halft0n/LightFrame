import { useTranslation } from "@/i18n/useTranslation";

export function UpdateChecker() {
  const { t } = useTranslation();

  const handleOpenReleasePage = () => {
    window.open("https://github.com/halft0n/LightFrame/releases", "_blank");
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
        </div>
        <button
          type="button"
          onClick={handleOpenReleasePage}
          className="rounded-lg border border-neutral-200 px-4 py-2 text-sm font-medium text-neutral-700 transition-colors hover:bg-neutral-100 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
        >
          {t("updates.check")}
        </button>
      </div>
    </section>
  );
}
