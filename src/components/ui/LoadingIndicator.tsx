import { useTranslation } from "@/i18n/useTranslation";

interface LoadingIndicatorProps {
  label?: string;
  className?: string;
  size?: "sm" | "md";
}

export function LoadingIndicator({ label, className = "", size = "md" }: LoadingIndicatorProps) {
  const { t } = useTranslation();
  const text = label ?? t("gallery.loading");

  return (
    <div
      role="status"
      aria-live="polite"
      aria-busy="true"
      className={`flex items-center justify-center gap-2 ${className}`}
    >
      <div
        className={`loading-shimmer-bar shimmer ${size === "sm" ? "h-2 w-16" : ""}`}
        aria-hidden="true"
      />
      <span className={`text-neutral-500 dark:text-neutral-400 ${size === "sm" ? "text-xs" : "text-sm"}`}>
        {text}
      </span>
    </div>
  );
}
