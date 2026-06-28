import { useState } from "react";
import type { MediaItem } from "@/lib/tauri";
import { getThumbnailUrl } from "@/lib/tauri";

interface PhotoCardProps {
  item: MediaItem;
  selected: boolean;
  onSelect: (id: number) => void;
}

function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export function PhotoCard({ item, selected, onSelect }: PhotoCardProps) {
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState(false);

  const isVideo = item.media_type === "Video";

  return (
    <button
      type="button"
      onClick={() => onSelect(item.id)}
      className={`photo-card group relative aspect-square w-full overflow-hidden rounded-lg bg-neutral-800 text-left transition-shadow ${
        selected ? "ring-2 ring-blue-500 ring-offset-2 ring-offset-neutral-950" : ""
      }`}
    >
      {!loaded && !error && (
        <div className="absolute inset-0 shimmer" aria-hidden="true" />
      )}

      {error ? (
        <div className="absolute inset-0 flex items-center justify-center bg-neutral-800 text-neutral-500">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            className="h-10 w-10"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909M3.75 21h16.5A2.25 2.25 0 0022.5 18.75V5.25A2.25 2.25 0 0020.25 3H3.75A2.25 2.25 0 001.5 5.25v13.5A2.25 2.25 0 003.75 21z"
            />
            <path strokeLinecap="round" strokeLinejoin="round" d="M9 9.75h.008v.008H9V9.75z" />
          </svg>
        </div>
      ) : (
        <img
          src={getThumbnailUrl(item.id, "small")}
          alt={item.filename}
          loading="lazy"
          decoding="async"
          onLoad={() => setLoaded(true)}
          onError={() => setError(true)}
          className={`h-full w-full object-cover transition-transform duration-200 group-hover:scale-105 ${
            loaded ? "opacity-100" : "opacity-0"
          }`}
        />
      )}

      {isVideo && item.duration_sec != null && (
        <span className="absolute bottom-2 right-2 rounded bg-black/70 px-1.5 py-0.5 text-xs font-medium text-white">
          {formatDuration(item.duration_sec)}
        </span>
      )}

      <div className="photo-card-overlay absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 to-transparent px-2 pb-2 pt-8 opacity-0 transition-opacity duration-200 group-hover:opacity-100">
        <p className="truncate text-xs text-white">{item.filename}</p>
      </div>
    </button>
  );
}
