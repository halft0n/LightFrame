import type {
  MediaItem,
  TimelineGroup,
  WatchedFolder,
  ScanProgress,
  MediaNeighbors,
  DuplicateGroup,
  DedupScanResult,
  LocationGroup,
  LocationStats,
  Album,
} from "./tauri";

const MOCK_COUNT = 36;

const MOCK_LOCATIONS: { country: string; city: string; lat: number; lon: number }[] = [
  { country: "CN", city: "Beijing", lat: 39.9042, lon: 116.4074 },
  { country: "CN", city: "Shanghai", lat: 31.2304, lon: 121.4737 },
  { country: "JP", city: "Tokyo", lat: 35.6762, lon: 139.6503 },
  { country: "JP", city: "Kyoto", lat: 35.0116, lon: 135.7681 },
  { country: "US", city: "New York", lat: 40.7128, lon: -74.006 },
];

function makeMockMedia(id: number): MediaItem {
  const types = ["Photo", "Photo", "Photo", "Photo", "Screenshot", "Video"] as const;
  const extensions = ["jpg", "png", "heic", "webp", "png", "mp4"];
  const typeIdx = id % types.length;
  const w = 800 + (id * 137) % 3200;
  const h = 600 + (id * 89) % 2400;
  const day = new Date();
  day.setDate(day.getDate() - Math.floor(id / 4));

  const hasGps = id % 3 !== 0;
  const loc = hasGps ? MOCK_LOCATIONS[id % MOCK_LOCATIONS.length] : null;

  return {
    id,
    path: `/mock/photos/IMG_${String(id).padStart(4, "0")}.${extensions[typeIdx]}`,
    filename: `IMG_${String(id).padStart(4, "0")}.${extensions[typeIdx]}`,
    media_type: types[typeIdx],
    size_bytes: 500_000 + (id * 12345) % 10_000_000,
    width: w,
    height: h,
    created_at: day.toISOString(),
    modified_at: day.toISOString(),
    duration_sec: types[typeIdx] === "Video" ? 10 + (id % 120) : null,
    blake3_hash: `mock_hash_${id}`,
    latitude: loc?.lat ?? null,
    longitude: loc?.lon ?? null,
  };
}

const allMedia: MediaItem[] = Array.from({ length: MOCK_COUNT }, (_, i) => makeMockMedia(i + 1));

const mockDeletedIds = new Set<number>([10, 11]);

function visibleMedia(): MediaItem[] {
  return allMedia.filter((m) => !mockDeletedIds.has(m.id));
}

const mockFolders: WatchedFolder[] = [
  { id: 1, path: "/mock/photos", media_count: MOCK_COUNT, last_scan: new Date().toISOString(), scan_status: "complete" },
];

export function getMockThumbnailUrl(id: number, size: "micro" | "small" | "large"): string {
  const dim = size === "micro" ? 64 : size === "small" ? 256 : 1024;
  const seed = id * 7 + 42;
  return `https://picsum.photos/seed/${seed}/${dim}/${dim}`;
}

export function getMockOriginalUrl(path: string): string {
  const id = parseInt(path.match(/(\d+)\./)?.[1] ?? "1", 10);
  return `https://picsum.photos/seed/${id * 7 + 42}/1920/1280`;
}

export async function mockListWatchedFolders(): Promise<WatchedFolder[]> {
  return mockFolders;
}

export async function mockAddWatchedFolder(path: string): Promise<WatchedFolder> {
  const folder: WatchedFolder = {
    id: mockFolders.length + 1,
    path,
    media_count: 0,
    last_scan: null,
    scan_status: "idle",
  };
  mockFolders.push(folder);
  return folder;
}

export async function mockRemoveWatchedFolder(id: number): Promise<void> {
  const idx = mockFolders.findIndex((f) => f.id === id);
  if (idx >= 0) mockFolders.splice(idx, 1);
}

export async function mockGetMediaList(offset: number, limit: number): Promise<MediaItem[]> {
  return visibleMedia().slice(offset, offset + limit);
}

export async function mockGetMediaCount(): Promise<number> {
  return visibleMedia().length;
}

export async function mockGetMediaByType(
  mediaType: MediaItem["media_type"],
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return visibleMedia()
    .filter((m) => m.media_type === mediaType)
    .slice(offset, offset + limit);
}

export async function mockGetMediaCountByType(
  mediaType: MediaItem["media_type"],
): Promise<number> {
  return visibleMedia().filter((m) => m.media_type === mediaType).length;
}

export async function mockGetMediaById(id: number): Promise<MediaItem | null> {
  return allMedia.find((m) => m.id === id) ?? null;
}

export async function mockGetTimelineGroups(limit = 200, offset = 0): Promise<TimelineGroup[]> {
  const paged = allMedia.slice(offset, offset + limit);
  const map = new Map<string, MediaItem[]>();

  for (const item of paged) {
    const d = (item.created_at ?? item.modified_at).slice(0, 10);
    const arr = map.get(d);
    if (arr) arr.push(item);
    else map.set(d, [item]);
  }

  return [...map.entries()].map(([date, media]) => ({
    date,
    count: media.length,
    media,
  }));
}

export async function mockGetMediaNeighbors(id: number): Promise<MediaNeighbors> {
  const idx = allMedia.findIndex((m) => m.id === id);
  return {
    prev_id: idx > 0 ? allMedia[idx - 1].id : null,
    next_id: idx < allMedia.length - 1 ? allMedia[idx + 1].id : null,
  };
}

export async function mockScanFolder(_folderId: number): Promise<void> {
  return;
}

export async function mockOnScanProgress(
  _callback: (progress: ScanProgress) => void,
): Promise<() => void> {
  return () => {};
}

const mockDedupGroups: DuplicateGroup[] = [
  {
    id: 1,
    match_type: "exact",
    created_at: new Date().toISOString(),
    members: [
      { media_id: 1, similarity: 1.0, path: "/mock/photos/IMG_0001.jpg", filename: "IMG_0001.jpg", size_bytes: 2_500_000, width: 3200, height: 2400, created_at: new Date().toISOString(), modified_at: new Date().toISOString() },
      { media_id: 5, similarity: 1.0, path: "/mock/photos/IMG_0005.png", filename: "IMG_0005.png", size_bytes: 2_500_000, width: 3200, height: 2400, created_at: new Date().toISOString(), modified_at: new Date().toISOString() },
    ],
  },
  {
    id: 2,
    match_type: "perceptual",
    created_at: new Date().toISOString(),
    members: [
      { media_id: 3, similarity: 0.92, path: "/mock/photos/IMG_0003.heic", filename: "IMG_0003.heic", size_bytes: 1_800_000, width: 2800, height: 2100, created_at: new Date().toISOString(), modified_at: new Date().toISOString() },
      { media_id: 7, similarity: 0.88, path: "/mock/photos/IMG_0007.jpg", filename: "IMG_0007.jpg", size_bytes: 1_200_000, width: 2800, height: 2100, created_at: new Date().toISOString(), modified_at: new Date().toISOString() },
      { media_id: 9, similarity: 0.85, path: "/mock/photos/IMG_0009.jpg", filename: "IMG_0009.jpg", size_bytes: 900_000, width: 2800, height: 2100, created_at: new Date().toISOString(), modified_at: new Date().toISOString() },
    ],
  },
];

export async function mockRunDedupScan(): Promise<DedupScanResult> {
  await new Promise((r) => setTimeout(r, 800));
  return { exact_groups: 1, perceptual_groups: 1, total_duplicates: 5 };
}

export async function mockGetDuplicateGroups(): Promise<DuplicateGroup[]> {
  return [...mockDedupGroups];
}

export async function mockGetDuplicateCount(): Promise<number> {
  return mockDedupGroups.length;
}

export async function mockResolveDuplicate(groupId: number): Promise<void> {
  const idx = mockDedupGroups.findIndex((g) => g.id === groupId);
  if (idx >= 0) mockDedupGroups.splice(idx, 1);
}

export async function mockDismissDuplicateGroup(groupId: number): Promise<void> {
  const idx = mockDedupGroups.findIndex((g) => g.id === groupId);
  if (idx >= 0) mockDedupGroups.splice(idx, 1);
}


function mediaMatchesLocation(item: MediaItem, country: string, city: string | null): boolean {
  if (item.latitude == null || item.longitude == null) return false;
  const idx = item.id % MOCK_LOCATIONS.length;
  const loc = MOCK_LOCATIONS[idx];
  if (loc.country !== country) return false;
  if (city == null) return true;
  return loc.city === city;
}

export async function mockGetLocationGroups(): Promise<LocationGroup[]> {
  const groups = new Map<string, LocationGroup>();

  for (const item of allMedia) {
    if (item.latitude == null) continue;
    const loc = MOCK_LOCATIONS[item.id % MOCK_LOCATIONS.length];
    const key = `${loc.country}|${loc.city}`;
    const existing = groups.get(key);
    if (existing) {
      existing.count += 1;
    } else {
      groups.set(key, {
        country: loc.country,
        city: loc.city,
        count: 1,
        sample_media_id: item.id,
      });
    }
  }

  return [...groups.values()].sort(
    (a, b) => a.country.localeCompare(b.country) || (a.city ?? "").localeCompare(b.city ?? ""),
  );
}

export async function mockGetMediaByLocation(
  country: string,
  city: string | null,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return allMedia
    .filter((m) => mediaMatchesLocation(m, country, city))
    .slice(offset, offset + limit);
}

export async function mockGetLocationStats(): Promise<LocationStats> {
  const withGps = allMedia.filter((m) => m.latitude != null);
  const groups = await mockGetLocationGroups();
  const countries = new Set(groups.map((g) => g.country)).size;
  return {
    total_with_gps: withGps.length,
    countries,
    cities: groups.length,
  };
}

const mockAlbums: Album[] = [
  {
    id: 1,
    name: "旅行",
    description: "旅行照片",
    cover_media_id: 1,
    media_count: 3,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 2,
    name: "家庭",
    description: null,
    cover_media_id: 5,
    media_count: 2,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
];

const mockAlbumItems = new Map<number, Set<number>>([
  [1, new Set([1, 2, 3])],
  [2, new Set([5, 6])],
]);

const mockFavoriteIds = new Set<number>([2, 4, 8]);

let nextAlbumId = 3;

export async function mockCreateAlbum(name: string, description?: string | null): Promise<Album> {
  const album: Album = {
    id: nextAlbumId++,
    name,
    description: description ?? null,
    cover_media_id: null,
    media_count: 0,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
  mockAlbums.push(album);
  mockAlbumItems.set(album.id, new Set());
  return album;
}

export async function mockDeleteAlbum(id: number): Promise<void> {
  const idx = mockAlbums.findIndex((a) => a.id === id);
  if (idx >= 0) mockAlbums.splice(idx, 1);
  mockAlbumItems.delete(id);
}

export async function mockListAlbums(): Promise<Album[]> {
  return mockAlbums.map((album) => {
    const items = mockAlbumItems.get(album.id);
    const mediaCount = items?.size ?? 0;
    const cover =
      album.cover_media_id ??
      (items && items.size > 0 ? [...items][0] : null);
    return { ...album, media_count: mediaCount, cover_media_id: cover };
  });
}

export async function mockAddToAlbum(albumId: number, mediaIds: number[]): Promise<void> {
  const items = mockAlbumItems.get(albumId) ?? new Set<number>();
  for (const id of mediaIds) {
    if (!mockDeletedIds.has(id)) items.add(id);
  }
  mockAlbumItems.set(albumId, items);
  const album = mockAlbums.find((a) => a.id === albumId);
  if (album) {
    album.media_count = items.size;
    if (album.cover_media_id == null && items.size > 0) {
      album.cover_media_id = [...items][0];
    }
    album.updated_at = new Date().toISOString();
  }
}

export async function mockRemoveFromAlbum(albumId: number, mediaId: number): Promise<void> {
  const items = mockAlbumItems.get(albumId);
  if (!items) return;
  items.delete(mediaId);
  const album = mockAlbums.find((a) => a.id === albumId);
  if (album) {
    album.media_count = items.size;
    if (album.cover_media_id === mediaId) {
      album.cover_media_id = items.size > 0 ? [...items][0] : null;
    }
    album.updated_at = new Date().toISOString();
  }
}

export async function mockGetAlbumMedia(
  albumId: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  const items = mockAlbumItems.get(albumId);
  if (!items) return [];
  const ids = [...items];
  return ids
    .map((id) => allMedia.find((m) => m.id === id))
    .filter((m): m is MediaItem => m != null && !mockDeletedIds.has(m.id))
    .slice(offset, offset + limit);
}

export async function mockToggleFavorite(mediaId: number): Promise<boolean> {
  if (mockFavoriteIds.has(mediaId)) {
    mockFavoriteIds.delete(mediaId);
    return false;
  }
  mockFavoriteIds.add(mediaId);
  return true;
}

export async function mockGetFavorites(offset: number, limit: number): Promise<MediaItem[]> {
  return visibleMedia()
    .filter((m) => mockFavoriteIds.has(m.id))
    .slice(offset, offset + limit);
}

export async function mockGetFavoritesCount(): Promise<number> {
  return visibleMedia().filter((m) => mockFavoriteIds.has(m.id)).length;
}

export async function mockDeleteMedia(mediaId: number): Promise<void> {
  mockDeletedIds.add(mediaId);
}

export async function mockGetDeletedMedia(): Promise<MediaItem[]> {
  return allMedia.filter((m) => mockDeletedIds.has(m.id));
}

export async function mockRestoreMedia(mediaId: number): Promise<void> {
  mockDeletedIds.delete(mediaId);
}

export async function mockPermanentlyDelete(mediaId: number): Promise<void> {
  mockDeletedIds.delete(mediaId);
  const idx = allMedia.findIndex((m) => m.id === mediaId);
  if (idx >= 0) allMedia.splice(idx, 1);
  mockFavoriteIds.delete(mediaId);
  for (const items of mockAlbumItems.values()) {
    items.delete(mediaId);
  }
}

function filterMockSearch(query: string): MediaItem[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return visibleMedia().filter((m) => m.filename.toLowerCase().includes(q));
}

export async function mockSearchMedia(
  query: string,
  limit: number,
  offset: number,
): Promise<MediaItem[]> {
  return filterMockSearch(query).slice(offset, offset + limit);
}

export async function mockSearchMediaCount(query: string): Promise<number> {
  return filterMockSearch(query).length;
}
