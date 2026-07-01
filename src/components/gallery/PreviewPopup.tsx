import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { convertFileSrc } from "@tauri-apps/api/core";

interface PreviewMediaItem {
  id: number;
  path: string;
  filename: string;
  media_type: string;
}

interface PreviewPopupProps {
  media: PreviewMediaItem | null;
  onClose: () => void;
}

export function PreviewPopup({ media, onClose }: PreviewPopupProps) {
  const [canDismiss, setCanDismiss] = useState(false);

  useEffect(() => {
    if (!media) return;
    setCanDismiss(false);
    const timer = window.setTimeout(() => setCanDismiss(true), 50);
    return () => window.clearTimeout(timer);
  }, [media]);

  useEffect(() => {
    if (!media) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [media, onClose]);

  useEffect(() => {
    if (!media || !canDismiss) return;
    const handlePointerUp = () => {
      onClose();
    };
    document.addEventListener("pointerup", handlePointerUp);
    return () => document.removeEventListener("pointerup", handlePointerUp);
  }, [media, canDismiss, onClose]);

  if (!media) return null;

  const handleBackdropDismiss = () => {
    if (canDismiss) {
      onClose();
    }
  };

  const isVideo = media.media_type === "Video";
  const src = isVideo
    ? convertFileSrc(media.path)
    : `original://localhost/${media.id}?size=small`;

  return createPortal(
    <div
      data-testid="preview-backdrop"
      className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={handleBackdropDismiss}
      onPointerCancel={handleBackdropDismiss}
    >
      <div
        data-testid="preview-popup"
        className="relative flex flex-col items-center gap-2"
        style={{ width: "min(400px, 60vw)" }}
        onClick={(e) => e.stopPropagation()}
      >
        {isVideo ? (
          <video
            src={src}
            autoPlay
            muted
            loop
            playsInline
            className="max-h-[60vh] w-full rounded-lg object-contain shadow-2xl"
          />
        ) : (
          <img
            src={src}
            alt={media.filename}
            className="max-h-[60vh] w-full rounded-lg object-contain shadow-2xl"
            draggable={false}
          />
        )}
        <span className="text-xs text-white/80 drop-shadow">
          {media.filename}
        </span>
      </div>
    </div>,
    document.body
  );
}
