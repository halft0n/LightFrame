import { useCallback, useEffect, useRef, useState } from "react";
import { getThumbnailUrl } from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";

interface VideoPlayerProps {
  src: string;
  mediaId: number;
  filmstripIds: number[];
  onNavigate?: (id: number) => void;
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export function VideoPlayer({ src, mediaId, filmstripIds, onNavigate }: VideoPlayerProps) {
  const { t } = useTranslation();
  const videoRef = useRef<HTMLVideoElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [playing, setPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);

  const togglePlay = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) {
      void video.play();
    } else {
      video.pause();
    }
  }, []);

  const seek = useCallback((time: number) => {
    const video = videoRef.current;
    if (!video) return;
    video.currentTime = Math.max(0, Math.min(time, video.duration || 0));
  }, []);

  const toggleFullscreen = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen();
    } else {
      void container.requestFullscreen();
    }
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === " ") {
        e.preventDefault();
        e.stopPropagation();
        togglePlay();
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        e.stopPropagation();
        seek(currentTime - 5);
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        e.stopPropagation();
        seek(currentTime + 5);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [togglePlay, seek, currentTime]);

  const handleProgressClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const ratio = (e.clientX - rect.left) / rect.width;
    seek(ratio * duration);
  };

  const handleFilmstripClick = (id: number, index: number) => {
    if (id === mediaId && duration > 0 && filmstripIds.length > 1) {
      const segment = duration / filmstripIds.length;
      seek(index * segment);
    } else {
      onNavigate?.(id);
    }
  };

  return (
    <div ref={containerRef} className="flex h-full flex-col bg-black">
      <div className="relative flex flex-1 items-center justify-center overflow-hidden">
        <video
          ref={videoRef}
          src={src}
          className="max-h-full max-w-full"
          onPlay={() => setPlaying(true)}
          onPause={() => setPlaying(false)}
          onTimeUpdate={() => setCurrentTime(videoRef.current?.currentTime ?? 0)}
          onLoadedMetadata={() => setDuration(videoRef.current?.duration ?? 0)}
          onClick={togglePlay}
        />
      </div>

      <div className="border-t border-white/10 px-4 py-3">
        <div
          role="slider"
          aria-valuemin={0}
          aria-valuemax={duration}
          aria-valuenow={currentTime}
          tabIndex={0}
          onClick={handleProgressClick}
          className="group relative h-1.5 cursor-pointer rounded-full bg-white/20"
        >
          <div
            className="absolute inset-y-0 left-0 rounded-full bg-blue-500"
            style={{ width: duration > 0 ? `${(currentTime / duration) * 100}%` : "0%" }}
          />
        </div>

        <div className="mt-3 flex items-center gap-3">
          <button
            type="button"
            onClick={togglePlay}
            className="rounded-lg px-3 py-1.5 text-sm text-neutral-200 transition hover:bg-white/10"
            aria-label={playing ? t("video.pause") : t("video.play")}
          >
            {playing ? t("video.pause") : t("video.play")}
          </button>

          <span className="text-xs text-neutral-400">
            {formatTime(currentTime)} / {formatTime(duration)}
          </span>

          <div className="flex flex-1 items-center gap-2">
            <button
              type="button"
              onClick={() => {
                const video = videoRef.current;
                if (!video) return;
                video.muted = !video.muted;
                setMuted(video.muted);
              }}
              className="rounded-lg px-2 py-1.5 text-xs text-neutral-300 transition hover:bg-white/10"
            >
              {muted ? t("video.unmute") : t("video.mute")}
            </button>
            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={volume}
              onChange={(e) => {
                const v = parseFloat(e.target.value);
                setVolume(v);
                if (videoRef.current) {
                  videoRef.current.volume = v;
                  if (v > 0) {
                    videoRef.current.muted = false;
                    setMuted(false);
                  }
                }
              }}
              className="w-20 accent-blue-500"
              aria-label={t("video.mute")}
            />
          </div>

          <button
            type="button"
            onClick={toggleFullscreen}
            className="rounded-lg px-3 py-1.5 text-sm text-neutral-300 transition hover:bg-white/10"
          >
            {t("video.fullscreen")}
          </button>
        </div>
      </div>

      {filmstripIds.length > 0 && (
        <div className="flex gap-2 overflow-x-auto border-t border-white/10 px-4 py-3">
          {filmstripIds.map((id, index) => (
            <button
              key={id}
              type="button"
              onClick={() => handleFilmstripClick(id, index)}
              className={`h-14 w-14 shrink-0 overflow-hidden rounded-md transition ${
                id === mediaId ? "ring-2 ring-blue-500" : "opacity-70 hover:opacity-100"
              }`}
            >
              <img
                src={getThumbnailUrl(id, "small")}
                alt=""
                className="h-full w-full object-cover"
              />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
