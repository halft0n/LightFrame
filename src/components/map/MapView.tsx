import { useEffect, useMemo, useRef, useState } from "react";
import { MapContainer, TileLayer, useMap } from "react-leaflet";
import L from "leaflet";
import "leaflet.markercluster";
import "leaflet.markercluster/dist/MarkerCluster.css";
import "leaflet.markercluster/dist/MarkerCluster.Default.css";
import iconRetinaUrl from "leaflet/dist/images/marker-icon-2x.png";
import iconUrl from "leaflet/dist/images/marker-icon.png";
import shadowUrl from "leaflet/dist/images/marker-shadow.png";
import { getMediaWithGeo, getThumbnailUrl, type MediaItem } from "@/lib/tauri";
import { escapeHtml } from "@/lib/escapeHtml";
import { useTranslation } from "@/i18n/useTranslation";
import { openViewer, useAppStore } from "@/store/appStore";
import { EmptyState } from "@/components/ui/EmptyState";

delete (L.Icon.Default.prototype as unknown as { _getIconUrl?: unknown })._getIconUrl;
L.Icon.Default.mergeOptions({
  iconUrl,
  iconRetinaUrl,
  shadowUrl,
});

const TILE_LAYERS = {
  light: {
    url: "https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png",
    attribution:
      '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
  },
  dark: {
    url: "https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png",
    attribution:
      '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/attributions">CARTO</a>',
  },
} as const;

function useResolvedTheme(): "light" | "dark" {
  const { theme } = useAppStore();
  const [resolved, setResolved] = useState<"light" | "dark">(() => {
    if (theme === "system") {
      return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    return theme;
  });

  useEffect(() => {
    if (theme !== "system") {
      setResolved(theme);
      return;
    }
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setResolved(mediaQuery.matches ? "dark" : "light");
    update();
    mediaQuery.addEventListener("change", update);
    return () => mediaQuery.removeEventListener("change", update);
  }, [theme]);

  return resolved;
}

interface PhotoMarker {
  id: number;
  lat: number;
  lng: number;
  filename: string;
}

interface PhotoClusterLayerProps {
  markers: PhotoMarker[];
  onSelect: (id: number) => void;
}

function PhotoClusterLayer({ markers, onSelect }: PhotoClusterLayerProps) {
  const map = useMap();
  const fittedRef = useRef(false);
  const onSelectRef = useRef(onSelect);
  onSelectRef.current = onSelect;

  useEffect(() => {
    fittedRef.current = false;
  }, [markers]);

  useEffect(() => {
    const cluster = L.markerClusterGroup();
    for (const marker of markers) {
      const popup = L.popup({ maxWidth: 220 }).setContent(
        `<div class="map-popup">
          <img src="${getThumbnailUrl(marker.id, "small")}" alt="" style="width:100%;border-radius:6px;aspect-ratio:4/3;object-fit:cover" />
          <p style="margin:6px 0 0;font-size:12px;font-weight:500">${escapeHtml(marker.filename)}</p>
        </div>`,
      );
      const m = L.marker([marker.lat, marker.lng]).bindPopup(popup);
      m.on("popupopen", () => onSelectRef.current(marker.id));
      cluster.addLayer(m);
    }

    map.addLayer(cluster);

    if (markers.length > 0 && !fittedRef.current) {
      map.fitBounds(cluster.getBounds().pad(0.1));
      fittedRef.current = true;
    }

    return () => {
      map.removeLayer(cluster);
    };
  }, [map, markers]);

  return null;
}

export function MapView() {
  const { t } = useTranslation();
  const resolvedTheme = useResolvedTheme();
  const [geoMedia, setGeoMedia] = useState<MediaItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [previewId, setPreviewId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void getMediaWithGeo(5000, 0)
      .then((media) => {
        if (!cancelled) setGeoMedia(media);
      })
      .catch((err) => {
        console.error("Failed to load geo media:", err);
        if (!cancelled) setGeoMedia([]);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const markers = useMemo(
    (): PhotoMarker[] =>
      geoMedia
        .filter((m) => m.latitude != null && m.longitude != null)
        .map((m) => ({
          id: m.id,
          lat: m.latitude!,
          lng: m.longitude!,
          filename: m.filename,
        })),
    [geoMedia],
  );

  const previewMedia = previewId != null ? geoMedia.find((m) => m.id === previewId) : null;
  const tileLayer = TILE_LAYERS[resolvedTheme];

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  if (geoMedia.length === 0) {
    return (
      <EmptyState
        variant="photos"
        title={t("map.noPhotos")}
        description={t("map.noPhotosHint")}
      />
    );
  }

  return (
    <div className="page-enter flex flex-1 flex-col overflow-hidden">
      <div className="flex shrink-0 items-center justify-between border-b border-neutral-200/70 px-4 py-3 dark:border-neutral-800/70">
        <div>
          <h1 className="text-base font-semibold text-neutral-900 dark:text-neutral-100">
            {t("map.title")}
          </h1>
          <p className="text-sm text-neutral-500">{t("map.photoCount", { count: geoMedia.length })}</p>
        </div>
      </div>

      <div className="relative min-h-0 flex-1">
        <MapContainer center={[20, 0]} zoom={2} className="h-full w-full" scrollWheelZoom>
          <TileLayer
            key={resolvedTheme}
            url={tileLayer.url}
            attribution={tileLayer.attribution}
          />
          <PhotoClusterLayer markers={markers} onSelect={setPreviewId} />
        </MapContainer>

        {previewMedia && (
          <div className="absolute bottom-4 left-4 z-[1000] w-56 overflow-hidden rounded-xl border border-neutral-200 bg-white shadow-xl dark:border-neutral-700 dark:bg-neutral-900">
            <button
              type="button"
              onClick={() => setPreviewId(null)}
              className="absolute right-2 top-2 z-10 rounded-full bg-black/50 px-1.5 py-0.5 text-xs text-white"
              aria-label={t("viewer.close")}
            >
              ×
            </button>
            <button
              type="button"
              onClick={() => openViewer(previewMedia.id)}
              className="block w-full text-left"
            >
              <img
                src={getThumbnailUrl(previewMedia.id, "small")}
                alt={previewMedia.filename}
                className="aspect-[4/3] w-full object-cover"
              />
              <div className="px-3 py-2">
                <p className="truncate text-sm font-medium text-neutral-900 dark:text-neutral-100">
                  {previewMedia.filename}
                </p>
                <p className="text-xs text-neutral-500">{t("map.openPhoto")}</p>
              </div>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
