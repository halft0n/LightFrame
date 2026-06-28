import { useTranslation } from "@/i18n/useTranslation";

export function MainContent() {
  const { t } = useTranslation();

  return (
    <div className="flex-1 flex items-center justify-center text-neutral-500">
      <div className="text-center space-y-4">
        <div className="text-6xl">📷</div>
        <p className="text-lg">{t("main.welcome")}</p>
        <p className="text-sm text-neutral-600">{t("main.addFolder")}</p>
      </div>
    </div>
  );
}
