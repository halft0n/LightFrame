import type { MediaItem } from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";

interface InfoPanelProps {
  media: MediaItem;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
  return `${(bytes / 1073741824).toFixed(1)} GB`;
}

function formatMediaDate(item: MediaItem, locale: string): string {
  const raw = item.created_at ?? item.modified_at;
  if (!raw) return "—";
  const date = new Date(raw);
  return new Intl.DateTimeFormat(locale === "zh-CN" ? "zh-CN" : "en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function formatHash(hash: string): string {
  if (hash.length <= 16) return hash;
  return `${hash.slice(0, 16)}…`;
}

function formatPerceptualHash(value: number): string {
  return `0x${value.toString(16).padStart(16, "0")}`;
}

function InfoRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div>
      <dt className="text-neutral-400">{label}</dt>
      <dd className="mt-0.5 break-all text-neutral-100">{value}</dd>
    </div>
  );
}

export function InfoPanel({ media }: InfoPanelProps) {
  const { t, locale } = useTranslation();

  const cameraParts = [media.camera_make, media.camera_model].filter(Boolean);
  const cameraInfo = cameraParts.length > 0 ? cameraParts.join(" ") : null;
  const hasGps = media.latitude != null && media.longitude != null;

  return (
    <aside
      role="complementary"
      aria-label={t("viewer.info")}
      className="info-panel-slide-in absolute inset-y-0 right-0 z-10 w-80 overflow-y-auto border-l border-white/10 bg-black/70 p-4 text-sm shadow-2xl backdrop-blur-sm"
    >
      <h2 className="mb-4 text-base font-semibold text-white">{t("viewer.info")}</h2>
      <dl className="space-y-3">
        <InfoRow label={t("viewer.filename")} value={media.filename} />
        <InfoRow label={t("viewer.path")} value={media.path} />
        {media.width != null && media.height != null && (
          <InfoRow
            label={t("viewer.dimensions")}
            value={`${media.width} × ${media.height}`}
          />
        )}
        <InfoRow label={t("viewer.size")} value={formatFileSize(media.size_bytes)} />
        <InfoRow label={t("viewer.type")} value={media.media_type} />
        {cameraInfo && <InfoRow label={t("viewer.camera")} value={cameraInfo} />}
        <InfoRow label={t("viewer.date")} value={formatMediaDate(media, locale)} />
        {hasGps && (
          <InfoRow
            label={t("viewer.gps")}
            value={`${media.latitude!.toFixed(6)}, ${media.longitude!.toFixed(6)}`}
          />
        )}
        {media.blake3_hash && (
          <InfoRow label={t("viewer.blake3")} value={formatHash(media.blake3_hash)} />
        )}
        {media.dhash != null && (
          <InfoRow label={t("viewer.dhash")} value={formatPerceptualHash(media.dhash)} />
        )}
        {media.phash != null && (
          <InfoRow label={t("viewer.phash")} value={formatPerceptualHash(media.phash)} />
        )}
      </dl>
    </aside>
  );
}
