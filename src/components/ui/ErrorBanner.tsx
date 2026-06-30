import { useTranslation } from "@/i18n/useTranslation";

interface ErrorBannerProps {
  message: string;
  onRetry?: () => void;
  className?: string;
}

export function ErrorBanner({
  message,
  onRetry,
  className = "",
}: ErrorBannerProps) {
  const { t } = useTranslation();

  return (
    <div
      role="alert"
      className={`flex flex-wrap items-center justify-between gap-3 border-b border-red-200/80 bg-red-50 px-4 py-2 text-sm text-red-900 dark:border-red-900/50 dark:bg-red-950/40 dark:text-red-200 ${className}`}
    >
      <p>{message}</p>
      {onRetry && (
        <button
          type="button"
          onClick={onRetry}
          className="shrink-0 rounded-md bg-red-600 px-3 py-1 text-xs font-medium text-white transition hover:bg-red-500 active:bg-red-700"
        >
          {t("search.retry")}
        </button>
      )}
    </div>
  );
}
