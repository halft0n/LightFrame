import { useCallback, useEffect, useRef, useState } from "react";
import {
  getMediaById,
  getOriginalUrl,
  getThumbnailUrl,
  type MediaItem,
} from "@/lib/tauri";
import { isTypingTarget } from "@/lib/keyboard";
import {
  closeSlideshow,
  nextSlideshow,
  prevSlideshow,
  setSlideshowSpeed,
  useAppStore,
  type SlideshowSpeed,
} from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const SPEEDS: SlideshowSpeed[] = [3, 5, 10];

export function SlideshowView() {
  const { t } = useTranslation();
  const { slideshowMediaIds, slideshowIndex, slideshowSpeed } = useAppStore();
  const [media, setMedia] = useState<MediaItem | null>(null);
  const [playing, setPlaying] = useState(true);
  const [transitionKey, setTransitionKey] = useState(0);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const currentId = slideshowMediaIds[slideshowIndex] ?? null;
  const isSingle = slideshowMediaIds.length <= 1;
  const isEmpty = slideshowMediaIds.length === 0;

  useEffect(() => {
    if (currentId == null) {
      setMedia(null);
      return;
    }
    let cancelled = false;
    void getMediaById(currentId).then((item) => {
      if (!cancelled) {
        setMedia(item);
        setTransitionKey((k) => k + 1);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [currentId]);

  useEffect(() => {
    if (!playing || isEmpty) return;
    const timer = window.setInterval(() => {
      nextSlideshow();
    }, slideshowSpeed * 1000);
    return () => window.clearInterval(timer);
  }, [playing, slideshowSpeed, slideshowIndex, slideshowMediaIds.length, isEmpty]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isTypingTarget(e.target)) return;

      if (e.key === "Escape") {
        e.preventDefault();
        if (isFullscreen && document.fullscreenElement) {
          void document.exitFullscreen();
        } else {
          closeSlideshow();
        }
        return;
      }

      if (e.key === " " || e.code === "Space") {
        e.preventDefault();
        setPlaying((p) => !p);
        return;
      }

      if (e.key === "ArrowLeft") {
        e.preventDefault();
        prevSlideshow();
        return;
      }

      if (e.key === "ArrowRight") {
        e.preventDefault();
        nextSlideshow();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isFullscreen]);

  useEffect(() => {
    const onFullscreenChange = () => {
      setIsFullscreen(Boolean(document.fullscreenElement));
    };
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () => document.removeEventListener("fullscreenchange", onFullscreenChange);
  }, []);

  const toggleFullscreen = useCallback(async () => {
    const el = containerRef.current;
    if (!el) return;
    if (document.fullscreenElement) {
      await document.exitFullscreen();
    } else {
      await el.requestFullscreen();
    }
  }, []);

  if (isEmpty) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center bg-neutral-950 text-neutral-300">
        <p>{t("slideshow.noPhotos")}</p>
        <button
          type="button"
          onClick={closeSlideshow}
          className="mt-4 rounded-lg bg-white/10 px-4 py-2 text-sm hover:bg-white/20"
        >
          {t("slideshow.exit")}
        </button>
      </div>
    );
  }

  const isVideo = media?.media_type === "Video";
  const imageSrc = media
    ? isVideo
      ? getOriginalUrl(media.path)
      : getThumbnailUrl(media.id, "large")
    : "";

  return (
    <div
      ref={containerRef}
      className="slideshow-view flex min-h-0 flex-1 flex-col overflow-hidden bg-neutral-950 text-white"
      role="dialog"
      aria-modal="true"
      aria-label={t("slideshow.title")}
    >
      <div className="flex shrink-0 items-center justify-between border-b border-white/10 px-4 py-2">
        <button
          type="button"
          onClick={closeSlideshow}
          className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10"
        >
          {t("slideshow.exit")}
        </button>
        <span className="text-sm tabular-nums text-neutral-400">
          {slideshowIndex + 1} / {slideshowMediaIds.length}
        </span>
        <button
          type="button"
          onClick={() => void toggleFullscreen()}
          className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10"
          aria-label={t("slideshow.fullscreen")}
        >
          {isFullscreen ? t("slideshow.exitFullscreen") : t("slideshow.fullscreen")}
        </button>
      </div>

      <div className="relative flex flex-1 items-center justify-center overflow-hidden">
        {!isSingle && (
          <button
            type="button"
            onClick={prevSlideshow}
            className="absolute left-4 z-10 rounded-full bg-black/40 p-3 text-2xl transition hover:bg-black/60"
            aria-label={t("slideshow.previous")}
          >
            ‹
          </button>
        )}

        <div key={transitionKey} className="slideshow-slide flex max-h-full max-w-full items-center justify-center p-4">
          {media && isVideo ? (
            <video
              src={imageSrc}
              autoPlay
              muted
              playsInline
              className="max-h-full max-w-full object-contain"
            />
          ) : media ? (
            <img
              src={imageSrc}
              alt={media.filename}
              className="max-h-full max-w-full select-none object-contain"
              draggable={false}
            />
          ) : (
            <p className="text-neutral-500">{t("gallery.loading")}</p>
          )}
        </div>

        {!isSingle && (
          <button
            type="button"
            onClick={nextSlideshow}
            className="absolute right-4 z-10 rounded-full bg-black/40 p-3 text-2xl transition hover:bg-black/60"
            aria-label={t("slideshow.next")}
          >
            ›
          </button>
        )}
      </div>

      <div className="flex shrink-0 flex-wrap items-center justify-center gap-3 border-t border-white/10 px-4 py-3">
        <button
          type="button"
          onClick={() => setPlaying((p) => !p)}
          className="rounded-lg bg-white/10 px-4 py-2 text-sm font-medium transition hover:bg-white/20"
          aria-label={playing ? t("slideshow.pause") : t("slideshow.play")}
        >
          {playing ? t("slideshow.pause") : t("slideshow.play")}
        </button>

        <div className="flex items-center gap-1 rounded-lg bg-white/5 p-1">
          {SPEEDS.map((speed) => (
            <button
              key={speed}
              type="button"
              onClick={() => setSlideshowSpeed(speed)}
              className={`rounded-md px-3 py-1.5 text-xs font-medium transition ${
                slideshowSpeed === speed
                  ? "bg-white/20 text-white"
                  : "text-neutral-400 hover:text-neutral-200"
              }`}
            >
              {t("slideshow.speedSeconds", { seconds: speed })}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
