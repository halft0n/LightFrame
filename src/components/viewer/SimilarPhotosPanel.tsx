import { useCallback, useEffect, useState } from "react";
import {
  findSimilarPhotos,
  computeClipEmbedding,
  getThumbnailUrl,
  type SimilarPhoto,
} from "@/lib/tauri";
import { openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";
import { localizeError } from "@/lib/errors";

interface SimilarPhotosPanelProps {
  mediaId: number;
  onClose: () => void;
}

export function SimilarPhotosPanel({
  mediaId,
  onClose,
}: SimilarPhotosPanelProps) {
  const { t } = useTranslation();
  const [photos, setPhotos] = useState<SimilarPhoto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadSimilar = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const results = await findSimilarPhotos(mediaId, 24);
      setPhotos(results);
    } catch (e) {
      setError(localizeError(e, t));
      setPhotos([]);
    } finally {
      setLoading(false);
    }
  }, [mediaId, t]);

  useEffect(() => {
    void loadSimilar();
  }, [loadSimilar]);

  const handleRetry = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await computeClipEmbedding(mediaId);
      await loadSimilar();
    } catch (e) {
      setError(localizeError(e, t));
      setLoading(false);
    }
  }, [mediaId, loadSimilar, t]);

  return (
    <aside
      className="absolute right-0 top-0 z-20 flex h-full w-72 flex-col border-l border-white/10 bg-neutral-950/95 backdrop-blur-sm sm:w-80"
      aria-label={t("similar.title")}
    >
      <div className="flex shrink-0 items-center justify-between border-b border-white/10 px-4 py-3">
        <h2 className="text-sm font-semibold text-neutral-100">
          {t("similar.title")}
        </h2>
        <button
          type="button"
          onClick={onClose}
          className="rounded-lg px-2 py-1 text-neutral-400 transition hover:bg-white/10 hover:text-white"
          aria-label={t("viewer.close")}
        >
          ×
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-3">
        {loading && (
          <p className="py-8 text-center text-sm text-neutral-400">
            {t("similar.computing")}
          </p>
        )}

        {!loading && error && (
          <div className="space-y-3 py-4 text-center">
            <p className="text-sm text-red-400">{error}</p>
            <button
              type="button"
              onClick={() => void handleRetry()}
              className="rounded-lg bg-white/10 px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-white/20"
            >
              {t("similar.retry")}
            </button>
          </div>
        )}

        {!loading && !error && photos.length === 0 && (
          <p className="py-8 text-center text-sm text-neutral-400">
            {t("similar.empty")}
          </p>
        )}

        {!loading && !error && photos.length > 0 && (
          <div className="grid grid-cols-2 gap-2">
            {photos.map((photo) => (
              <button
                key={photo.media_id}
                type="button"
                onClick={() => openViewer(photo.media_id)}
                className="group relative aspect-square overflow-hidden rounded-md"
                aria-label={`${Math.round(photo.similarity * 100)}% ${t("similar.match")}`}
              >
                <img
                  src={getThumbnailUrl(photo.media_id, "small")}
                  alt=""
                  className="h-full w-full object-cover transition group-hover:scale-105"
                />
                <span className="absolute bottom-1 right-1 rounded bg-black/70 px-1.5 py-0.5 text-xs font-medium text-white">
                  {Math.round(photo.similarity * 100)}%
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </aside>
  );
}
