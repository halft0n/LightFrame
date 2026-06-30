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
  setSlideshowIndex,
  setSlideshowSpeed,
  useAppStore,
  type SlideshowSpeed,
} from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const SPEEDS: SlideshowSpeed[] = [3, 5, 10];
const CONTROLS_HIDE_MS = 2000;

function mediaSrc(media: MediaItem): string {
  return media.media_type === "Video"
    ? getOriginalUrl(media.path)
    : getThumbnailUrl(media.id, "large");
}

export function SlideshowView() {
  const { t } = useTranslation();
  const { slideshowMediaIds, slideshowIndex, slideshowSpeed } = useAppStore();
  const [currentMedia, setCurrentMedia] = useState<MediaItem | null>(null);
  const [previousMedia, setPreviousMedia] = useState<MediaItem | null>(null);
  const [playing, setPlaying] = useState(true);
  const [shuffleMode, setShuffleMode] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [progress, setProgress] = useState(0);
  const [slideError, setSlideError] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const hideTimerRef = useRef<number | null>(null);
  const shuffleHistoryRef = useRef<number[]>([]);
  const currentMediaRef = useRef<MediaItem | null>(null);
  currentMediaRef.current = currentMedia;

  const currentId = slideshowMediaIds[slideshowIndex] ?? null;
  const isSingle = slideshowMediaIds.length <= 1;
  const isEmpty = slideshowMediaIds.length === 0;

  const showControls = useCallback(() => {
    setControlsVisible(true);
    if (hideTimerRef.current != null) {
      window.clearTimeout(hideTimerRef.current);
    }
    hideTimerRef.current = window.setTimeout(() => {
      setControlsVisible(false);
      hideTimerRef.current = null;
    }, CONTROLS_HIDE_MS);
  }, []);

  const goToRandomNext = useCallback(() => {
    if (slideshowMediaIds.length <= 1) return;
    shuffleHistoryRef.current.push(slideshowIndex);
    let next = slideshowIndex;
    while (next === slideshowIndex) {
      next = Math.floor(Math.random() * slideshowMediaIds.length);
    }
    setSlideshowIndex(next);
  }, [slideshowIndex, slideshowMediaIds.length]);

  const goNext = useCallback(() => {
    if (shuffleMode) {
      goToRandomNext();
    } else {
      nextSlideshow();
    }
  }, [shuffleMode, goToRandomNext]);

  const goPrev = useCallback(() => {
    if (shuffleMode) {
      const history = shuffleHistoryRef.current;
      if (history.length === 0) {
        goToRandomNext();
        return;
      }
      const prev = history.pop()!;
      setSlideshowIndex(prev);
    } else {
      prevSlideshow();
    }
  }, [shuffleMode, goToRandomNext]);

  useEffect(() => {
    showControls();
    return () => {
      if (hideTimerRef.current != null) {
        window.clearTimeout(hideTimerRef.current);
      }
    };
  }, [showControls]);

  useEffect(() => {
    shuffleHistoryRef.current = [];
  }, [shuffleMode]);

  useEffect(() => {
    if (currentId == null) {
      setCurrentMedia(null);
      setPreviousMedia(null);
      return;
    }
    let cancelled = false;
    setSlideError(false);
    void getMediaById(currentId)
      .then((item) => {
        if (cancelled || !item) return;
        setPreviousMedia(currentMediaRef.current);
        setCurrentMedia(item);
      })
      .catch(() => {
        if (!cancelled) setSlideError(true);
      });
    return () => {
      cancelled = true;
    };
  }, [currentId]);

  const toggleFullscreen = useCallback(async () => {
    const el = containerRef.current;
    if (!el) return;
    if (document.fullscreenElement) {
      await document.exitFullscreen();
    } else {
      await el.requestFullscreen();
    }
  }, []);

  useEffect(() => {
    if (!playing || isEmpty) return;
    const slideStart = Date.now();
    setProgress(0);

    const timer = window.setInterval(() => {
      goNext();
    }, slideshowSpeed * 1000);

    const progressTimer = window.setInterval(() => {
      const elapsed = Date.now() - slideStart;
      setProgress(Math.min(100, (elapsed / (slideshowSpeed * 1000)) * 100));
    }, 50);

    return () => {
      window.clearInterval(timer);
      window.clearInterval(progressTimer);
    };
  }, [
    playing,
    slideshowSpeed,
    slideshowIndex,
    slideshowMediaIds.length,
    isEmpty,
    goNext,
  ]);

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
        showControls();
        return;
      }

      if (e.key === "ArrowLeft") {
        e.preventDefault();
        goPrev();
        showControls();
        return;
      }

      if (e.key === "ArrowRight") {
        e.preventDefault();
        goNext();
        showControls();
        return;
      }

      if (e.key === "f" || e.key === "F") {
        e.preventDefault();
        void toggleFullscreen();
        showControls();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isFullscreen, goNext, goPrev, showControls, toggleFullscreen]);

  useEffect(() => {
    const onFullscreenChange = () => {
      setIsFullscreen(Boolean(document.fullscreenElement));
    };
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () =>
      document.removeEventListener("fullscreenchange", onFullscreenChange);
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

  const renderSlide = (media: MediaItem, fading: "in" | "out" | "static") => {
    const src = mediaSrc(media);
    const fadeClass =
      fading === "in"
        ? "slideshow-crossfade-in"
        : fading === "out"
          ? "slideshow-crossfade-out"
          : "";

    if (media.media_type === "Video") {
      return (
        <video
          key={media.id}
          src={src}
          autoPlay
          muted
          playsInline
          className={`max-h-full max-w-full object-contain ${fadeClass}`}
        />
      );
    }

    return (
      <img
        key={media.id}
        src={src}
        alt={media.filename}
        className={`max-h-full max-w-full select-none object-contain ${fadeClass}`}
        draggable={false}
      />
    );
  };

  return (
    <div
      ref={containerRef}
      className="slideshow-view relative flex min-h-0 flex-1 flex-col overflow-hidden bg-neutral-950 text-white"
      role="dialog"
      aria-modal="true"
      aria-label={t("slideshow.title")}
      tabIndex={-1}
      onContextMenu={(e) => e.preventDefault()}
      onMouseMove={showControls}
      onFocus={showControls}
    >
      <div
        className="pointer-events-none absolute inset-x-0 top-0 z-30 h-0.5 bg-white/10"
        aria-hidden="true"
      >
        <div
          className="h-full bg-white/80 transition-[width] duration-75 ease-linear"
          style={{ width: `${progress}%` }}
        />
      </div>

      <div
        className={`pointer-events-none absolute inset-0 z-20 flex flex-col transition-opacity duration-300 ${
          controlsVisible ? "opacity-100" : "opacity-0"
        }`}
      >
        <div className="pointer-events-auto flex shrink-0 items-center justify-between bg-gradient-to-b from-black/60 to-transparent px-4 pb-6 pt-3">
          <button
            type="button"
            onClick={closeSlideshow}
            className="rounded-lg px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-white/10"
          >
            {t("slideshow.exit")}
          </button>
          <span className="text-sm tabular-nums text-neutral-300">
            {slideshowIndex + 1} / {slideshowMediaIds.length}
          </span>
          <button
            type="button"
            onClick={() => void toggleFullscreen()}
            className="rounded-lg px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-white/10"
            aria-label={
              isFullscreen
                ? t("slideshow.exitFullscreen")
                : t("slideshow.fullscreen")
            }
          >
            {isFullscreen
              ? t("slideshow.exitFullscreen")
              : t("slideshow.fullscreen")}
          </button>
        </div>

        <div className="pointer-events-none flex flex-1 items-center justify-between px-4">
          {!isSingle && (
            <button
              type="button"
              onClick={goPrev}
              className="pointer-events-auto rounded-full bg-black/50 p-3 text-2xl backdrop-blur-sm transition hover:bg-black/70"
              aria-label={t("slideshow.previous")}
            >
              ‹
            </button>
          )}
          <div className="flex-1" />
          {!isSingle && (
            <button
              type="button"
              onClick={goNext}
              className="pointer-events-auto rounded-full bg-black/50 p-3 text-2xl backdrop-blur-sm transition hover:bg-black/70"
              aria-label={t("slideshow.next")}
            >
              ›
            </button>
          )}
        </div>

        <div className="pointer-events-auto flex shrink-0 flex-wrap items-center justify-center gap-3 bg-gradient-to-t from-black/60 to-transparent px-4 pb-4 pt-6">
          <button
            type="button"
            onClick={() => setPlaying((p) => !p)}
            className="rounded-full bg-black/50 px-5 py-2.5 text-sm font-medium backdrop-blur-sm transition hover:bg-black/70"
            aria-label={playing ? t("slideshow.pause") : t("slideshow.play")}
          >
            {playing ? t("slideshow.pause") : t("slideshow.play")}
          </button>

          <button
            type="button"
            onClick={() => setShuffleMode((s) => !s)}
            className={`rounded-full px-4 py-2.5 text-sm font-medium backdrop-blur-sm transition ${
              shuffleMode
                ? "bg-white/25 text-white"
                : "bg-black/50 text-neutral-300 hover:bg-black/70 hover:text-white"
            }`}
            aria-label={
              shuffleMode ? t("slideshow.sequential") : t("slideshow.shuffle")
            }
            aria-pressed={shuffleMode}
          >
            {shuffleMode ? t("slideshow.shuffleOn") : t("slideshow.shuffleOff")}
          </button>

          <div className="flex items-center gap-1 rounded-full bg-black/50 p-1 backdrop-blur-sm">
            {SPEEDS.map((speed) => (
              <button
                key={speed}
                type="button"
                onClick={() => setSlideshowSpeed(speed)}
                className={`rounded-full px-3 py-1.5 text-xs font-medium transition ${
                  slideshowSpeed === speed
                    ? "bg-white/25 text-white"
                    : "text-neutral-400 hover:text-neutral-200"
                }`}
              >
                {t("slideshow.speedSeconds", { seconds: speed })}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="relative flex flex-1 items-center justify-center overflow-hidden p-4">
        {previousMedia &&
          currentMedia &&
          previousMedia.id !== currentMedia.id && (
            <div className="absolute inset-0 flex items-center justify-center p-4">
              {renderSlide(previousMedia, "out")}
            </div>
          )}
        {currentMedia ? (
          <div className="relative flex max-h-full max-w-full items-center justify-center">
            {renderSlide(
              currentMedia,
              previousMedia && previousMedia.id !== currentMedia.id
                ? "in"
                : "static",
            )}
          </div>
        ) : slideError ? (
          <div className="flex flex-col items-center gap-3 text-neutral-400">
            <span className="text-4xl">⚠</span>
            <p>{t("viewer.loadError")}</p>
            <button
              type="button"
              onClick={goNext}
              className="rounded-lg bg-white/10 px-4 py-1.5 text-sm transition hover:bg-white/20"
            >
              {t("viewer.next")}
            </button>
          </div>
        ) : (
          <p className="text-neutral-500">{t("gallery.loading")}</p>
        )}
      </div>
    </div>
  );
}
