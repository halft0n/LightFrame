import { useCallback, useEffect, useState } from "react";
import {
  findSimilarPhotos,
  computeClipEmbedding,
  getThumbnailUrl,
  type SimilarPhoto,
} from "@/lib/tauri";
import { openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

interface SimilarPhotosViewProps {
  mediaId: number;
  onBack?: () => void;
}

export function SimilarPhotosView({ mediaId, onBack }: SimilarPhotosViewProps) {
  const { t } = useTranslation();
  const [photos, setPhotos] = useState<SimilarPhoto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadSimilar = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const results = await findSimilarPhotos(mediaId, 48);
      setPhotos(results);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhotos([]);
    } finally {
      setLoading(false);
    }
  }, [mediaId]);

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
      setError(e instanceof Error ? e.message : String(e));
      setLoading(false);
    }
  }, [mediaId, loadSimilar]);

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden bg-neutral-950 text-white">
      <div className="flex shrink-0 items-center gap-3 border-b border-white/10 px-4 py-3">
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            className="rounded-lg px-2 py-1.5 text-lg leading-none text-neutral-300 transition hover:bg-white/10"
            aria-label={t("viewer.back")}
          >
            ‹
          </button>
        )}
        <h1 className="text-sm font-semibold text-neutral-100">{t("similar.title")}</h1>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {loading && (
          <p className="py-12 text-center text-sm text-neutral-400">{t("similar.computing")}</p>
        )}

        {!loading && error && (
          <div className="space-y-3 py-8 text-center">
            <p className="text-sm text-red-400">{error}</p>
            <button
              type="button"
              onClick={() => void handleRetry()}
              className="rounded-lg bg-white/10 px-4 py-2 text-sm text-neutral-200 transition hover:bg-white/20"
            >
              {t("similar.retry")}
            </button>
          </div>
        )}

        {!loading && !error && photos.length === 0 && (
          <p className="py-12 text-center text-sm text-neutral-400">{t("similar.empty")}</p>
        )}

        {!loading && !error && photos.length > 0 && (
          <div className="grid grid-cols-3 gap-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6">
            {photos.map((photo) => (
              <button
                key={photo.media_id}
                type="button"
                onClick={() => openViewer(photo.media_id)}
                className="group relative aspect-square overflow-hidden rounded-lg"
                aria-label={`${Math.round(photo.similarity * 100)}% ${t("similar.match")}`}
              >
                <img
                  src={getThumbnailUrl(photo.media_id, "small")}
                  alt=""
                  className="h-full w-full object-cover transition group-hover:scale-105"
                />
                <span className="absolute bottom-1.5 right-1.5 rounded bg-black/70 px-2 py-0.5 text-xs font-medium text-white">
                  {Math.round(photo.similarity * 100)}%
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
