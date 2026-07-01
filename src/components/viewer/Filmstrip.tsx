import { memo, useEffect, useRef, useCallback } from "react";
import { getThumbnailUrl } from "@/lib/tauri";
import type { MediaItem } from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";

export interface FilmstripProps {
  items: MediaItem[];
  currentId: number;
  onNavigate: (id: number) => void;
  visible?: boolean;
}

export const Filmstrip = memo(function Filmstrip({
  items,
  currentId,
  onNavigate,
  visible = true,
}: FilmstripProps) {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!visible) return;
    const el = containerRef.current;
    if (!el) return;
    const active = el.querySelector(`[data-id="${currentId}"]`);
    active?.scrollIntoView({
      inline: "center",
      block: "nearest",
      behavior: "smooth",
    });
  }, [currentId, visible]);

  const handleClick = useCallback(
    (id: number) => {
      if (id !== currentId) {
        onNavigate(id);
      }
    },
    [currentId, onNavigate],
  );

  if (!visible || items.length === 0) return null;

  return (
    <div
      ref={containerRef}
      className="flex shrink-0 gap-2 overflow-x-auto border-t border-white/10 px-4 py-3"
      role="tablist"
      aria-label={t("viewer.filmstrip")}
    >
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="tab"
          aria-selected={item.id === currentId}
          data-id={String(item.id)}
          onClick={() => handleClick(item.id)}
          className={`h-16 w-16 shrink-0 overflow-hidden rounded-md transition ${
            item.id === currentId
              ? "ring-2 ring-blue-500"
              : "opacity-70 hover:opacity-100"
          }`}
          aria-label={item.filename}
        >
          <img
            src={getThumbnailUrl(item.id, "micro")}
            alt=""
            className="h-full w-full object-cover"
            style={{ imageRendering: "pixelated" }}
          />
        </button>
      ))}
    </div>
  );
});
