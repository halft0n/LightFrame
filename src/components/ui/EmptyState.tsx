import type { ReactNode } from "react";

type EmptyStateVariant = "photos" | "welcome" | "folder";

interface EmptyStateProps {
  variant?: EmptyStateVariant;
  title: string;
  description?: string;
  action?: ReactNode;
}

function EmptyIllustration({ variant }: { variant: EmptyStateVariant }) {
  if (variant === "welcome") {
    return (
      <div className="empty-state-icon" aria-hidden="true">
        <svg viewBox="0 0 120 120" className="h-28 w-28" fill="none">
          <defs>
            <linearGradient id="welcomeGrad" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="var(--accent-primary)" stopOpacity="0.35" />
              <stop offset="100%" stopColor="var(--accent-secondary)" stopOpacity="0.15" />
            </linearGradient>
          </defs>
          <circle cx="60" cy="60" r="52" fill="url(#welcomeGrad)" />
          <rect x="28" y="38" width="64" height="48" rx="8" fill="var(--bg-tertiary)" stroke="var(--border-color)" strokeWidth="1.5" />
          <circle cx="44" cy="54" r="8" fill="var(--accent-primary)" opacity="0.7" />
          <path d="M28 72 L48 58 L62 68 L82 50 L92 58 L92 86 L28 86 Z" fill="var(--accent-primary)" opacity="0.25" />
          <circle cx="88" cy="32" r="14" fill="var(--accent-primary)" opacity="0.2" />
          <path d="M88 24 L88 40 M80 32 L96 32" stroke="var(--accent-primary)" strokeWidth="2" strokeLinecap="round" opacity="0.6" />
        </svg>
      </div>
    );
  }

  if (variant === "folder") {
    return (
      <div className="empty-state-icon" aria-hidden="true">
        <svg viewBox="0 0 120 120" className="h-24 w-24" fill="none">
          <path
            d="M16 36 C16 30 20 26 26 26 H48 L56 34 H94 C100 34 104 38 104 44 V88 C104 94 100 98 94 98 H26 C20 98 16 94 16 88 Z"
            fill="var(--bg-tertiary)"
            stroke="var(--border-color)"
            strokeWidth="1.5"
          />
          <path d="M40 58 H80 M40 70 H68" stroke="var(--text-muted)" strokeWidth="2" strokeLinecap="round" opacity="0.5" />
        </svg>
      </div>
    );
  }

  return (
    <div className="empty-state-icon" aria-hidden="true">
      <svg viewBox="0 0 120 120" className="h-24 w-24" fill="none">
        <rect x="24" y="32" width="72" height="56" rx="10" fill="var(--bg-tertiary)" stroke="var(--border-color)" strokeWidth="1.5" />
        <circle cx="42" cy="50" r="7" fill="var(--accent-primary)" opacity="0.5" />
        <path d="M24 72 L44 56 L58 66 L78 46 L96 60 L96 88 L24 88 Z" fill="var(--accent-primary)" opacity="0.2" />
        <circle cx="88" cy="28" r="10" stroke="var(--text-muted)" strokeWidth="1.5" strokeDasharray="3 3" opacity="0.4" />
      </svg>
    </div>
  );
}

export function EmptyState({ variant = "photos", title, description, action }: EmptyStateProps) {
  const titleId = "empty-state-title";

  return (
    <div
      role="status"
      aria-labelledby={titleId}
      className="page-enter flex flex-1 flex-col items-center justify-center px-6 py-16 text-center"
    >
      <EmptyIllustration variant={variant} />
      <h2
        id={titleId}
        className="mt-6 text-xl font-semibold tracking-tight text-neutral-800 dark:text-neutral-100"
      >
        {title}
      </h2>
      {description && (
        <p className="mt-2 max-w-sm text-sm leading-relaxed text-neutral-500 dark:text-neutral-400">
          {description}
        </p>
      )}
      {action && <div className="mt-6">{action}</div>}
    </div>
  );
}
