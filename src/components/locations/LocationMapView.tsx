import { useEffect, useMemo, useRef, useState } from "react";
import { MapContainer, TileLayer, useMap } from "react-leaflet";
import L from "leaflet";
import "leaflet.markercluster";
import "leaflet.markercluster/dist/MarkerCluster.css";
import "leaflet.markercluster/dist/MarkerCluster.Default.css";
import iconRetinaUrl from "leaflet/dist/images/marker-icon-2x.png";
import iconUrl from "leaflet/dist/images/marker-icon.png";
import shadowUrl from "leaflet/dist/images/marker-shadow.png";
import { getMediaById, type LocationGroup } from "@/lib/tauri";
import { escapeHtml } from "@/lib/escapeHtml";
import { useTranslation } from "@/i18n/useTranslation";
import { useAppStore } from "@/store/appStore";

delete (L.Icon.Default.prototype as unknown as { _getIconUrl?: unknown })
  ._getIconUrl;
L.Icon.Default.mergeOptions({
  iconUrl,
  iconRetinaUrl,
  shadowUrl,
});

interface LocationMarker {
  group: LocationGroup;
  lat: number;
  lng: number;
  label: string;
}

function locationLabel(country: string, city: string | null): string {
  if (city) return `${city}, ${country}`;
  return country;
}

function useResolvedTheme(): "light" | "dark" {
  const { theme } = useAppStore();
  const [resolved, setResolved] = useState<"light" | "dark">(() => {
    if (theme === "system") {
      return window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light";
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

interface MarkerClusterLayerProps {
  markers: LocationMarker[];
  photosLabel: (count: number) => string;
}

function MarkerClusterLayer({ markers, photosLabel }: MarkerClusterLayerProps) {
  const map = useMap();
  const fittedRef = useRef(false);

  useEffect(() => {
    fittedRef.current = false;
  }, [markers]);

  useEffect(() => {
    const cluster = L.markerClusterGroup();
    for (const marker of markers) {
      const popup = L.popup().setContent(
        `<strong>${escapeHtml(marker.label)}</strong><br/>${escapeHtml(photosLabel(marker.group.count))}`,
      );
      L.marker([marker.lat, marker.lng]).bindPopup(popup).addTo(cluster);
    }

    map.addLayer(cluster);

    if (markers.length > 0 && !fittedRef.current) {
      map.fitBounds(cluster.getBounds().pad(0.1));
      fittedRef.current = true;
    }

    return () => {
      map.removeLayer(cluster);
    };
  }, [map, markers, photosLabel]);

  return null;
}

interface LocationMapViewProps {
  groups: LocationGroup[];
}

export function LocationMapView({ groups }: LocationMapViewProps) {
  const { t } = useTranslation();
  const resolvedTheme = useResolvedTheme();
  const [markers, setMarkers] = useState<LocationMarker[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    void (async () => {
      try {
        const results = await Promise.all(
          groups.map(async (group) => {
            try {
              const media = await getMediaById(group.sample_media_id);
              if (media?.latitude == null || media?.longitude == null)
                return null;
              return {
                group,
                lat: media.latitude,
                lng: media.longitude,
                label: locationLabel(group.country, group.city),
              };
            } catch (err) {
              console.error(
                `Failed to load coordinates for location group ${group.country}/${group.city ?? ""}:`,
                err,
              );
              return null;
            }
          }),
        );

        if (!cancelled) {
          setMarkers(
            results.filter((item): item is LocationMarker => item !== null),
          );
        }
      } catch (err) {
        console.error("Failed to load map markers:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [groups]);

  const photosLabel = useMemo(
    () => (count: number) => t("locations.photos", { count }),
    [t],
  );

  const tileLayer = TILE_LAYERS[resolvedTheme];
  const defaultCenter: [number, number] = [20, 0];
  const defaultZoom = 2;

  if (loading) {
    return (
      <div className="flex h-full min-h-[320px] flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  if (markers.length === 0) {
    return (
      <div className="flex h-full min-h-[320px] flex-1 items-center justify-center text-neutral-500">
        <p>{t("locations.noLocations")}</p>
      </div>
    );
  }

  return (
    <div className="h-full min-h-0 flex-1">
      <MapContainer
        center={defaultCenter}
        zoom={defaultZoom}
        className="h-full w-full rounded-none"
        scrollWheelZoom
      >
        <TileLayer
          key={resolvedTheme}
          url={tileLayer.url}
          attribution={tileLayer.attribution}
        />
        <MarkerClusterLayer markers={markers} photosLabel={photosLabel} />
      </MapContainer>
    </div>
  );
}
