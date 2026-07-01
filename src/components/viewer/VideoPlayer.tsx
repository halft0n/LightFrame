import { useCallback, useEffect, useRef, useState } from "react";
import { getThumbnailUrl } from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";
import { TrimBar } from "./TrimBar";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

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

export function VideoPlayer({
  src,
  mediaId,
  filmstripIds,
  onNavigate,
}: VideoPlayerProps) {
  const { t } = useTranslation();
  const videoRef = useRef<HTMLVideoElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [playing, setPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const currentTimeRef = useRef(currentTime);
  currentTimeRef.current = currentTime;
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);
  const [trimMode, setTrimMode] = useState(false);
  const [trimIn, setTrimIn] = useState(0);
  const [trimOut, setTrimOut] = useState(0);

  // Load trim params on mount
  useEffect(() => {
    invoke<{ video_trim_in_sec?: number; video_trim_out_sec?: number } | null>(
      "get_video_trim",
      { mediaId }
    ).then((params) => {
      if (params) {
        setTrimIn(params.video_trim_in_sec ?? 0);
        setTrimOut(params.video_trim_out_sec ?? duration);
      }
    }).catch((err) => console.warn("Failed to load video trim params:", err));
  }, [mediaId]); // eslint-disable-line react-hooks/exhaustive-deps

  // When duration loads and no trim saved, set trimOut to duration
  useEffect(() => {
    if (duration > 0 && trimOut === 0) {
      setTrimOut(duration);
    }
  }, [duration]); // eslint-disable-line react-hooks/exhaustive-deps

  const trimActive =
    duration > 0 && trimOut > 0 && (trimIn > 0 || trimOut < duration);

  // Constrain playback to saved trim range (not only while editing)
  useEffect(() => {
    if (!trimActive || !videoRef.current) return;
    const video = videoRef.current;
    const handleTimeUpdate = () => {
      if (video.currentTime >= trimOut) {
        video.pause();
        video.currentTime = trimIn;
      } else if (video.currentTime < trimIn) {
        video.currentTime = trimIn;
      }
    };
    video.addEventListener("timeupdate", handleTimeUpdate);
    return () => video.removeEventListener("timeupdate", handleTimeUpdate);
  }, [trimActive, trimIn, trimOut]);

  const handleApplyTrim = useCallback(() => {
    invoke("save_video_trim", {
      mediaId,
      trimInSec: trimIn,
      trimOutSec: trimOut,
    }).catch((err) => console.warn("Failed to save video trim:", err));
  }, [mediaId, trimIn, trimOut]);

  const handleExportTrim = useCallback(async () => {
    let outputDir: string | null = null;

    try {
      const outputPath = await save({
        defaultPath: "trimmed_video.mp4",
        filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm"] }],
      });
      if (outputPath) {
        const sep = outputPath.includes("\\") ? "\\" : "/";
        const lastSep = outputPath.lastIndexOf(sep);
        outputDir = lastSep >= 0 ? outputPath.slice(0, lastSep) : outputPath;
      }
    } catch (err) {
      console.warn("Save dialog unavailable, falling back to folder picker:", err);
    }

    if (!outputDir) {
      try {
        outputDir = (await open({ directory: true, multiple: false })) ?? null;
      } catch (err) {
        console.warn("Folder picker unavailable:", err);
      }
    }

    if (!outputDir) return;

    invoke<string>("export_trimmed_video", {
      mediaId,
      outputDir,
    }).catch((err) => console.warn("Failed to export trimmed video:", err));
  }, [mediaId]);

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
        seek(currentTimeRef.current - 5);
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        e.stopPropagation();
        seek(currentTimeRef.current + 5);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [togglePlay, seek]);

  const handleProgressClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const ratio = (e.clientX - rect.left) / rect.width;
    seek(ratio * duration);
  };

  const handleProgressKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (duration <= 0) return;
    const step = duration * 0.05;
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      seek(currentTime - step);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      seek(currentTime + step);
    } else if (e.key === "Home") {
      e.preventDefault();
      seek(0);
    } else if (e.key === "End") {
      e.preventDefault();
      seek(duration);
    }
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
          onTimeUpdate={() =>
            setCurrentTime(videoRef.current?.currentTime ?? 0)
          }
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
          onKeyDown={handleProgressKeyDown}
          className="group relative h-1.5 cursor-pointer rounded-full bg-white/20"
        >
          <div
            className="absolute inset-y-0 left-0 rounded-full bg-blue-500"
            style={{
              width: duration > 0 ? `${(currentTime / duration) * 100}%` : "0%",
            }}
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

          <button
            type="button"
            onClick={() => setTrimMode((v) => !v)}
            className={`rounded-lg px-3 py-1.5 text-sm transition ${
              trimMode
                ? "bg-blue-600 text-white"
                : "text-neutral-300 hover:bg-white/10"
            }`}
          >
            Trim
          </button>
        </div>

        {trimMode && (
          <div className="mt-3">
            <TrimBar
              duration={duration}
              trimIn={trimIn}
              trimOut={trimOut}
              onTrimInChange={setTrimIn}
              onTrimOutChange={setTrimOut}
              onApply={handleApplyTrim}
              onExport={handleExportTrim}
            />
          </div>
        )}
      </div>

      {filmstripIds.length > 0 && (
        <div className="flex gap-2 overflow-x-auto border-t border-white/10 px-4 py-3">
          {filmstripIds.map((id, index) => (
            <button
              key={id}
              type="button"
              onClick={() => handleFilmstripClick(id, index)}
              className={`h-14 w-14 shrink-0 overflow-hidden rounded-md transition ${
                id === mediaId
                  ? "ring-2 ring-blue-500"
                  : "opacity-70 hover:opacity-100"
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
