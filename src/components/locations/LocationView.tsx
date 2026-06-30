import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { PhotoCard } from "@/components/gallery/PhotoCard";
import {
  getLocationGroups,
  getLocationStats,
  getMediaByLocation,
  getThumbnailUrl,
  type LocationGroup,
  type LocationStats,
  type MediaItem,
} from "@/lib/tauri";
import { openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";
import { LocationMapView } from "@/components/locations/LocationMapView";

const MIN_COLUMN_WIDTH = 160;
const GAP = 3;
const PAGE_SIZE = 60;
const CARD_PAGE_SIZE = 20;

interface SelectedLocation {
  country: string;
  city: string | null;
  label: string;
}

function groupByCountry(groups: LocationGroup[]): Map<string, LocationGroup[]> {
  const map = new Map<string, LocationGroup[]>();
  for (const group of groups) {
    const list = map.get(group.country);
    if (list) list.push(group);
    else map.set(group.country, [group]);
  }
  return map;
}

function locationLabel(country: string, city: string | null): string {
  if (city) return `${city}, ${country}`;
  return country;
}

type LocationViewMode = "grid" | "map";

function GridViewIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      className={className}
      aria-hidden="true"
    >
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  );
}

function MapViewIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      className={className}
      aria-hidden="true"
    >
      <path d="M9 4 3 6v14l6-2 6 2 6-2V4l-6 2-6-2-6 2Z" />
      <path d="M9 4v14M15 6v14" />
    </svg>
  );
}

interface LocationGroupCardProps {
  group: LocationGroup;
  countLabel: string;
  onSelect: (group: LocationGroup) => void;
}

const LocationGroupCard = memo(function LocationGroupCard({
  group,
  countLabel,
  onSelect,
}: LocationGroupCardProps) {
  return (
    <button
      type="button"
      onClick={() => onSelect(group)}
      className="card-list-item group flex items-center gap-3 rounded-lg border border-neutral-200/80 dark:border-neutral-800 bg-neutral-100 dark:bg-neutral-900/50 p-3 text-left transition hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800/80"
    >
      <div className="h-16 w-16 shrink-0 overflow-hidden rounded-md bg-neutral-800">
        <img
          src={getThumbnailUrl(group.sample_media_id, "small")}
          alt=""
          className="card-thumb h-full w-full object-cover transition group-hover:scale-105"
          loading="lazy"
          decoding="async"
        />
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate font-medium text-neutral-800 dark:text-neutral-100">
          {group.city ?? group.country}
        </p>
        <p className="text-sm text-neutral-500">{countLabel}</p>
      </div>
    </button>
  );
});

export function LocationView() {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);
  const [groups, setGroups] = useState<LocationGroup[]>([]);
  const [stats, setStats] = useState<LocationStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<SelectedLocation | null>(null);
  const [media, setMedia] = useState<MediaItem[]>([]);
  const [mediaLoading, setMediaLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const [containerWidth, setContainerWidth] = useState(0);
  const [visibleCount, setVisibleCount] = useState(CARD_PAGE_SIZE);
  const [viewMode, setViewMode] = useState<LocationViewMode>("grid");

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );

  useEffect(() => {
    let cancelled = false;

    async function loadInitial() {
      setLoading(true);
      try {
        const [groupData, statsData] = await Promise.all([
          getLocationGroups(),
          getLocationStats(),
        ]);
        if (!cancelled) {
          setGroups(groupData);
          setStats(statsData);
          setVisibleCount(CARD_PAGE_SIZE);
        }
      } catch (err) {
        console.error("Failed to load location data:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    void loadInitial();
    return () => {
      cancelled = true;
    };
  }, []);

  const loadLocationMedia = useCallback(
    async (
      country: string,
      city: string | null,
      offset = 0,
      append = false,
    ) => {
      if (offset === 0) setMediaLoading(true);
      else setLoadingMore(true);
      try {
        const items = await getMediaByLocation(
          country,
          city,
          offset,
          PAGE_SIZE,
        );
        setMedia((prev) => (append ? [...prev, ...items] : items));
        setHasMore(items.length >= PAGE_SIZE);
      } catch (err) {
        console.error("Failed to load location media:", err);
      } finally {
        setMediaLoading(false);
        setLoadingMore(false);
      }
    },
    [],
  );

  const handleSelectGroup = useCallback(
    (group: LocationGroup) => {
      const label = locationLabel(group.country, group.city);
      setSelected({ country: group.country, city: group.city, label });
      void loadLocationMedia(group.country, group.city);
    },
    [loadLocationMedia],
  );

  const handleBack = useCallback(() => {
    setSelected(null);
    setMedia([]);
    setHasMore(false);
  }, []);

  const loadMore = useCallback(async () => {
    if (!selected || loadingMore || !hasMore) return;
    await loadLocationMedia(
      selected.country,
      selected.city,
      media.length,
      true,
    );
  }, [selected, loadingMore, hasMore, media.length, loadLocationMedia]);

  const handlePhotoSelect = useCallback(
    (id: number, _event: React.MouseEvent) => {
      openViewer(id);
    },
    [],
  );

  useEffect(() => {
    const el = parentRef.current;
    if (!el || !selected) return;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = el;
      if (scrollHeight - scrollTop - clientHeight < 200) {
        void loadMore();
      }
    };

    el.addEventListener("scroll", handleScroll, { passive: true });
    return () => el.removeEventListener("scroll", handleScroll);
  }, [loadMore, selected]);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) setContainerWidth(entry.contentRect.width);
    });
    observer.observe(el);
    setContainerWidth(el.clientWidth);
    return () => observer.disconnect();
  }, [selected]);

  const visibleGroups = useMemo(
    () => groups.slice(0, visibleCount),
    [groups, visibleCount],
  );
  const hasMoreCards = visibleCount < groups.length;
  const byCountry = useMemo(
    () => groupByCountry(visibleGroups),
    [visibleGroups],
  );

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  if (groups.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2 text-neutral-500">
        <p>{t("locations.noLocations")}</p>
        <p className="text-sm text-neutral-600">
          {t("locations.noLocationsHint")}
        </p>
      </div>
    );
  }

  if (selected) {
    return (
      <div className="flex flex-1 flex-col overflow-hidden">
        <div className="flex items-center gap-3 border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-2">
          <button
            type="button"
            onClick={handleBack}
            className="rounded-md px-2 py-1 text-sm text-neutral-500 dark:text-neutral-400 transition hover:bg-white/10 hover:text-neutral-200"
          >
            ← {t("locations.allPhotos")}
          </button>
          <span className="text-sm text-neutral-700 dark:text-neutral-300">
            {selected.label}
          </span>
          <span className="text-sm text-neutral-500">
            {t("gallery.count", { count: media.length })}
          </span>
        </div>

        <div ref={parentRef} className="flex-1 overflow-y-auto px-1 py-1">
          {mediaLoading ? (
            <div className="flex items-center justify-center py-12 text-neutral-500">
              <p>{t("gallery.loading")}</p>
            </div>
          ) : media.length === 0 ? (
            <div className="flex items-center justify-center py-12 text-neutral-500">
              <p>{t("gallery.noPhotos")}</p>
            </div>
          ) : (
            <div
              className="grid gap-[3px]"
              style={{
                gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
              }}
            >
              {media.map((item) => (
                <PhotoCard
                  key={item.id}
                  item={item}
                  selected={false}
                  selectedMediaIds={[]}
                  onSelect={handlePhotoSelect}
                  onOpen={openViewer}
                />
              ))}
            </div>
          )}
          {loadingMore && (
            <div className="py-4 text-center text-sm text-neutral-500">
              {t("gallery.loading")}
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {stats && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-3 text-sm text-neutral-500 dark:text-neutral-400">
          <span className="font-medium text-neutral-700 dark:text-neutral-200">
            {t("locations.title")}
          </span>
          <span>
            {t("locations.photosWithGps", { count: stats.total_with_gps })}
          </span>
          <span>{t("locations.countries", { count: stats.countries })}</span>
          <span>{t("locations.cities", { count: stats.cities })}</span>
          <div className="ml-auto flex items-center gap-1 rounded-lg border border-neutral-200/80 dark:border-neutral-700 p-0.5">
            <button
              type="button"
              onClick={() => setViewMode("grid")}
              title={t("locations.gridView")}
              aria-label={t("locations.gridView")}
              className={`rounded-md p-1.5 transition ${
                viewMode === "grid"
                  ? "bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-100"
                  : "text-neutral-500 hover:text-neutral-300 dark:text-neutral-400 dark:hover:text-neutral-200"
              }`}
            >
              <GridViewIcon className="h-4 w-4" />
            </button>
            <button
              type="button"
              onClick={() => setViewMode("map")}
              title={t("locations.mapView")}
              aria-label={t("locations.mapView")}
              className={`rounded-md p-1.5 transition ${
                viewMode === "map"
                  ? "bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-100"
                  : "text-neutral-500 hover:text-neutral-300 dark:text-neutral-400 dark:hover:text-neutral-200"
              }`}
            >
              <MapViewIcon className="h-4 w-4" />
            </button>
          </div>
        </div>
      )}

      {viewMode === "map" ? (
        <LocationMapView groups={groups} />
      ) : (
        <div ref={parentRef} className="flex-1 overflow-y-auto px-1 py-1">
          {[...byCountry.entries()].map(([country, cityGroups]) => (
            <section key={country} className="mb-6">
              <h2 className="mb-3 text-base font-semibold text-neutral-800 dark:text-neutral-100">
                {country}
              </h2>
              <div className="grid gap-[3px] sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                {cityGroups.map((group) => (
                  <LocationGroupCard
                    key={`${group.country}-${group.city ?? ""}`}
                    group={group}
                    countLabel={t("gallery.count", { count: group.count })}
                    onSelect={handleSelectGroup}
                  />
                ))}
              </div>
            </section>
          ))}
          {hasMoreCards && (
            <div className="flex justify-center pb-6">
              <button
                type="button"
                onClick={() => setVisibleCount((prev) => prev + CARD_PAGE_SIZE)}
                className="rounded-lg border border-neutral-200/80 dark:border-neutral-700 px-6 py-2 text-sm text-neutral-500 dark:text-neutral-400 transition hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800 hover:text-neutral-200"
              >
                {t("gallery.loadMore")} ({groups.length - visibleCount})
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
