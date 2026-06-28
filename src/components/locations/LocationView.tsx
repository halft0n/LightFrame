import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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

const MIN_COLUMN_WIDTH = 180;
const GAP = 12;
const PAGE_SIZE = 60;

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

  const columnCount = Math.max(
    1,
    Math.floor((containerWidth + GAP) / (MIN_COLUMN_WIDTH + GAP)),
  );

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void Promise.all([getLocationGroups(), getLocationStats()]).then(([groupData, statsData]) => {
      if (!cancelled) {
        setGroups(groupData);
        setStats(statsData);
        setLoading(false);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const loadLocationMedia = useCallback(
    async (country: string, city: string | null, offset = 0, append = false) => {
      if (offset === 0) setMediaLoading(true);
      else setLoadingMore(true);
      try {
        const items = await getMediaByLocation(country, city, offset, PAGE_SIZE);
        setMedia((prev) => (append ? [...prev, ...items] : items));
        setHasMore(items.length >= PAGE_SIZE);
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
    await loadLocationMedia(selected.country, selected.city, media.length, true);
  }, [selected, loadingMore, hasMore, media.length, loadLocationMedia]);

  useEffect(() => {
    const el = parentRef.current;
    if (!el || !selected) return;

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = el;
      if (scrollHeight - scrollTop - clientHeight < 200) {
        void loadMore();
      }
    };

    el.addEventListener("scroll", handleScroll);
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

  const byCountry = useMemo(() => groupByCountry(groups), [groups]);

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
        <p className="text-sm text-neutral-600">{t("locations.noLocationsHint")}</p>
      </div>
    );
  }

  if (selected) {
    return (
      <div className="flex flex-1 flex-col overflow-hidden">
        <div className="flex items-center gap-3 border-b border-neutral-800 px-4 py-2">
          <button
            type="button"
            onClick={handleBack}
            className="rounded-md px-2 py-1 text-sm text-neutral-400 transition hover:bg-white/10 hover:text-neutral-200"
          >
            ← {t("locations.allPhotos")}
          </button>
          <span className="text-sm text-neutral-300">{selected.label}</span>
          <span className="text-sm text-neutral-500">
            {t("gallery.count", { count: media.length })}
          </span>
        </div>

        <div ref={parentRef} className="flex-1 overflow-y-auto px-4 py-3">
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
              className="grid gap-3"
              style={{
                gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
              }}
            >
              {media.map((item) => (
                <PhotoCard
                  key={item.id}
                  item={item}
                  selected={false}
                  onSelect={() => openViewer(item.id)}
                  onOpen={openViewer}
                />
              ))}
            </div>
          )}
          {loadingMore && (
            <div className="py-4 text-center text-sm text-neutral-500">{t("gallery.loading")}</div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {stats && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-neutral-800 px-4 py-3 text-sm text-neutral-400">
          <span className="font-medium text-neutral-200">{t("locations.title")}</span>
          <span>{t("locations.photosWithGps", { count: stats.total_with_gps })}</span>
          <span>{t("locations.countries", { count: stats.countries })}</span>
          <span>{t("locations.cities", { count: stats.cities })}</span>
        </div>
      )}

      <div ref={parentRef} className="flex-1 overflow-y-auto px-4 py-3">
        {[...byCountry.entries()].map(([country, cityGroups]) => (
          <section key={country} className="mb-6">
            <h2 className="mb-3 text-base font-semibold text-neutral-100">{country}</h2>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
              {cityGroups.map((group) => (
                <button
                  key={`${group.country}-${group.city ?? ""}`}
                  type="button"
                  onClick={() => handleSelectGroup(group)}
                  className="group flex items-center gap-3 rounded-lg border border-neutral-800 bg-neutral-900/50 p-3 text-left transition hover:border-neutral-600 hover:bg-neutral-800/80"
                >
                  <div className="h-16 w-16 shrink-0 overflow-hidden rounded-md bg-neutral-800">
                    <img
                      src={getThumbnailUrl(group.sample_media_id, "small")}
                      alt=""
                      className="h-full w-full object-cover transition group-hover:scale-105"
                      loading="lazy"
                    />
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="truncate font-medium text-neutral-100">
                      {group.city ?? group.country}
                    </p>
                    <p className="text-sm text-neutral-500">
                      {t("gallery.count", { count: group.count })}
                    </p>
                  </div>
                </button>
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
