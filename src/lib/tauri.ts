import {
  getMockThumbnailUrl,
  getMockOriginalUrl,
  mockListWatchedFolders,
  mockAddWatchedFolder,
  mockRemoveWatchedFolder,
  mockGetMediaList,
  mockGetMediaCount,
  mockGetMediaById,
  mockGetTimelineGroups,
  mockGetMediaNeighbors,
  mockScanFolder,
  mockOnScanProgress,
  mockRunDedupScan,
  mockGetDuplicateGroups,
  mockGetDuplicateCount,
  mockResolveDuplicate,
  mockDismissDuplicateGroup,
  mockGetLocationGroups,
  mockGetMediaByLocation,
  mockGetLocationStats,
  mockCreateAlbum,
  mockDeleteAlbum,
  mockListAlbums,
  mockAddToAlbum,
  mockRemoveFromAlbum,
  mockGetAlbumMedia,
  mockToggleFavorite,
  mockGetFavorites,
  mockGetFavoritesCount,
  mockDeleteMedia,
  mockGetDeletedMedia,
  mockRestoreMedia,
  mockPermanentlyDelete,
  mockBatchDeleteMedia,
  mockBatchToggleFavorite,
  mockBatchRestoreMedia,
  mockBatchPermanentDelete,
  mockSearchMedia,
  mockSearchMediaCount,
  mockGetMediaByType,
  mockGetMediaCountByType,
  mockCreateSmartAlbum,
  mockListSmartAlbums,
  mockDeleteSmartAlbum,
  mockGetSmartAlbumMedia,
  mockGenerateMemories,
  mockListMemories,
  mockGetMemoryMedia,
  mockGetAiStatus,
  mockListPersons,
  mockGetPersonMedia,
  mockRenamePerson,
} from "./mock-data";
import { convertFileSrc } from "@tauri-apps/api/core";

export type MediaType =
  | "Photo"
  | "Video"
  | "Screenshot"
  | "LivePhoto"
  | "Raw"
  | "Unknown";

export type ScanStatus = "idle" | "scanning" | "complete" | "error";

export interface WatchedFolder {
  id: number;
  path: string;
  media_count: number;
  last_scan?: string | null;
  scan_status: ScanStatus;
}

export interface MediaItem {
  id: number;
  path: string;
  filename: string;
  media_type: MediaType;
  size_bytes: number;
  width?: number | null;
  height?: number | null;
  created_at?: string | null;
  modified_at: string;
  duration_sec?: number | null;
  blake3_hash?: string | null;
  latitude?: number | null;
  longitude?: number | null;
}

export interface ScanProgress {
  folder_id: number;
  scanned: number;
  total: number;
  status: ScanStatus;
}

export interface TimelineGroup {
  date: string;
  count: number;
  media: MediaItem[];
}

export interface MediaNeighbors {
  prev_id: number | null;
  next_id: number | null;
}

export interface DuplicateMember {
  media_id: number;
  similarity: number;
  path: string;
  filename: string;
  size_bytes: number;
  width?: number | null;
  height?: number | null;
  created_at?: string | null;
  modified_at: string;
}

export interface DuplicateGroup {
  id: number;
  match_type: "exact" | "perceptual";
  created_at: string;
  members: DuplicateMember[];
}

export interface DedupScanResult {
  exact_groups: number;
  perceptual_groups: number;
  total_duplicates: number;
}

export interface LocationGroup {
  country: string;
  city: string | null;
  count: number;
  sample_media_id: number;
}

export interface LocationStats {
  total_with_gps: number;
  countries: number;
  cities: number;
}

export interface Album {
  id: number;
  name: string;
  description: string | null;
  cover_media_id: number | null;
  media_count: number;
  created_at: string;
  updated_at: string;
}

export interface SmartAlbumRule {
  media_type?: string | null;
  date_from?: string | null;
  date_to?: string | null;
  country?: string | null;
  city?: string | null;
  is_favorite?: boolean | null;
  min_size?: number | null;
  has_gps?: boolean | null;
}

export interface SmartAlbum {
  id: number;
  name: string;
  icon: string | null;
  rule_json: string;
  media_count: number;
  created_at: string;
}

export interface Memory {
  id: number;
  title: string;
  subtitle: string | null;
  cover_media_id: number;
  media_count: number;
  date_from: string;
  date_to: string;
  created_at: string;
}

export interface AiStatus {
  python_available: boolean;
  clip_available: boolean;
  face_available: boolean;
  status_message: string;
}

export interface Person {
  id: number;
  name: string | null;
  face_count: number;
  cover_face_id: number | null;
  sample_media_ids: number[];
  created_at: string;
}

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export function getThumbnailUrl(id: number, size: "small" | "large" | "micro" = "small"): string {
  if (!isTauri) return getMockThumbnailUrl(id, size);
  return `thumb://localhost/${id}/${size}`;
}

export function getOriginalUrl(path: string): string {
  if (!isTauri) return getMockOriginalUrl(path);
  return convertFileSrc(path);
}

export async function addWatchedFolder(path: string): Promise<WatchedFolder> {
  if (!isTauri) return mockAddWatchedFolder(path);
  return tauriInvoke<WatchedFolder>("add_watched_folder", { path });
}

export async function removeWatchedFolder(id: number): Promise<void> {
  if (!isTauri) return mockRemoveWatchedFolder(id);
  return tauriInvoke("remove_watched_folder", { id });
}

export async function listWatchedFolders(): Promise<WatchedFolder[]> {
  if (!isTauri) return mockListWatchedFolders();
  return tauriInvoke<WatchedFolder[]>("list_watched_folders");
}

export async function getMediaList(offset: number, limit: number): Promise<MediaItem[]> {
  if (!isTauri) return mockGetMediaList(offset, limit);
  return tauriInvoke<MediaItem[]>("get_media_list", { offset, limit });
}

export async function getMediaCount(): Promise<number> {
  if (!isTauri) return mockGetMediaCount();
  return tauriInvoke<number>("get_media_count");
}

export async function getMediaByType(
  mediaType: MediaType,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  if (!isTauri) return mockGetMediaByType(mediaType, offset, limit);
  return tauriInvoke<MediaItem[]>("get_media_by_type", {
    mediaType,
    offset,
    limit,
  });
}

export async function getMediaCountByType(mediaType: MediaType): Promise<number> {
  if (!isTauri) return mockGetMediaCountByType(mediaType);
  return tauriInvoke<number>("get_media_count_by_type", { mediaType });
}

export async function getMediaById(id: number): Promise<MediaItem | null> {
  if (!isTauri) return mockGetMediaById(id);
  return tauriInvoke<MediaItem | null>("get_media_by_id", { id });
}

export async function getTimelineGroups(limit = 200, offset = 0): Promise<TimelineGroup[]> {
  if (!isTauri) return mockGetTimelineGroups(limit, offset);
  return tauriInvoke<TimelineGroup[]>("get_timeline_groups", { limit, offset });
}

export async function getMediaNeighbors(id: number): Promise<MediaNeighbors> {
  if (!isTauri) return mockGetMediaNeighbors(id);
  return tauriInvoke<MediaNeighbors>("get_media_neighbors", { id });
}

export async function scanFolder(folderId: number): Promise<void> {
  if (!isTauri) return mockScanFolder(folderId);
  return tauriInvoke("scan_folder", { folderId });
}

export async function onScanProgress(
  callback: (progress: ScanProgress) => void,
): Promise<() => void> {
  if (!isTauri) return mockOnScanProgress(callback);
  const { listen } = await import("@tauri-apps/api/event");
  return listen<ScanProgress>("scan-progress", (event) => {
    callback(event.payload);
  });
}

export async function runDedupScan(): Promise<DedupScanResult> {
  if (!isTauri) return mockRunDedupScan();
  return tauriInvoke<DedupScanResult>("run_dedup_scan");
}

export async function getDuplicateGroups(): Promise<DuplicateGroup[]> {
  if (!isTauri) return mockGetDuplicateGroups();
  return tauriInvoke<DuplicateGroup[]>("get_duplicate_groups");
}

export async function getDuplicateCount(): Promise<number> {
  if (!isTauri) return mockGetDuplicateCount();
  return tauriInvoke<number>("get_duplicate_count");
}

export async function resolveDuplicate(
  groupId: number,
  keepMediaId: number,
  deleteFiles: boolean,
): Promise<void> {
  if (!isTauri) return mockResolveDuplicate(groupId);
  return tauriInvoke("resolve_duplicate", {
    groupId,
    keepMediaId,
    deleteFiles,
  });
}

export async function dismissDuplicateGroup(groupId: number): Promise<void> {
  if (!isTauri) return mockDismissDuplicateGroup(groupId);
  return tauriInvoke("dismiss_duplicate_group", { groupId });
}

export async function getLocationGroups(): Promise<LocationGroup[]> {
  if (!isTauri) return mockGetLocationGroups();
  return tauriInvoke<LocationGroup[]>("get_location_groups");
}

export async function getMediaByLocation(
  country: string,
  city: string | null,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  if (!isTauri) return mockGetMediaByLocation(country, city, offset, limit);
  return tauriInvoke<MediaItem[]>("get_media_by_location", {
    country,
    city,
    offset,
    limit,
  });
}

export async function getLocationStats(): Promise<LocationStats> {
  if (!isTauri) return mockGetLocationStats();
  return tauriInvoke<LocationStats>("get_location_stats");
}

export async function createAlbum(
  name: string,
  description?: string | null,
): Promise<Album> {
  if (!isTauri) return mockCreateAlbum(name, description);
  return tauriInvoke<Album>("create_album", { name, description: description ?? null });
}

export async function deleteAlbum(id: number): Promise<void> {
  if (!isTauri) return mockDeleteAlbum(id);
  return tauriInvoke("delete_album", { id });
}

export async function listAlbums(): Promise<Album[]> {
  if (!isTauri) return mockListAlbums();
  return tauriInvoke<Album[]>("list_albums");
}

export async function addToAlbum(albumId: number, mediaIds: number[]): Promise<void> {
  if (!isTauri) return mockAddToAlbum(albumId, mediaIds);
  return tauriInvoke("add_to_album", { albumId, mediaIds });
}

export async function removeFromAlbum(albumId: number, mediaId: number): Promise<void> {
  if (!isTauri) return mockRemoveFromAlbum(albumId, mediaId);
  return tauriInvoke("remove_from_album", { albumId, mediaId });
}

export async function getAlbumMedia(
  albumId: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  if (!isTauri) return mockGetAlbumMedia(albumId, offset, limit);
  return tauriInvoke<MediaItem[]>("get_album_media", { albumId, offset, limit });
}

export async function toggleFavorite(mediaId: number): Promise<boolean> {
  if (!isTauri) return mockToggleFavorite(mediaId);
  return tauriInvoke<boolean>("toggle_favorite", { mediaId });
}

export async function getFavorites(offset: number, limit: number): Promise<MediaItem[]> {
  if (!isTauri) return mockGetFavorites(offset, limit);
  return tauriInvoke<MediaItem[]>("get_favorites", { offset, limit });
}

export async function getFavoritesCount(): Promise<number> {
  if (!isTauri) return mockGetFavoritesCount();
  return tauriInvoke<number>("get_favorites_count");
}

export async function deleteMedia(mediaId: number): Promise<void> {
  if (!isTauri) return mockDeleteMedia(mediaId);
  return tauriInvoke("delete_media", { mediaId });
}

export async function getDeletedMedia(): Promise<MediaItem[]> {
  if (!isTauri) return mockGetDeletedMedia();
  return tauriInvoke<MediaItem[]>("get_deleted_media");
}

export async function restoreMedia(mediaId: number): Promise<void> {
  if (!isTauri) return mockRestoreMedia(mediaId);
  return tauriInvoke("restore_media", { mediaId });
}

export async function permanentlyDelete(mediaId: number): Promise<void> {
  if (!isTauri) return mockPermanentlyDelete(mediaId);
  return tauriInvoke("permanently_delete", { mediaId });
}

export async function batchDeleteMedia(mediaIds: number[]): Promise<number> {
  if (!isTauri) return mockBatchDeleteMedia(mediaIds);
  return tauriInvoke<number>("batch_delete_media", { mediaIds });
}

export async function batchAddToAlbum(albumId: number, mediaIds: number[]): Promise<void> {
  if (!isTauri) return mockAddToAlbum(albumId, mediaIds);
  return tauriInvoke("batch_add_to_album", { albumId, mediaIds });
}

export async function batchToggleFavorite(
  mediaIds: number[],
  favorite: boolean,
): Promise<number> {
  if (!isTauri) return mockBatchToggleFavorite(mediaIds, favorite);
  return tauriInvoke<number>("batch_toggle_favorite", { mediaIds, favorite });
}

export async function batchRestoreMedia(mediaIds: number[]): Promise<number> {
  if (!isTauri) return mockBatchRestoreMedia(mediaIds);
  return tauriInvoke<number>("batch_restore_media", { mediaIds });
}

export async function batchPermanentDelete(mediaIds: number[]): Promise<number> {
  if (!isTauri) return mockBatchPermanentDelete(mediaIds);
  return tauriInvoke<number>("batch_permanent_delete", { mediaIds });
}

export async function searchMedia(
  query: string,
  limit: number,
  offset: number,
): Promise<MediaItem[]> {
  if (!isTauri) return mockSearchMedia(query, limit, offset);
  return tauriInvoke<MediaItem[]>("search_media", { query, limit, offset });
}

export async function searchMediaCount(query: string): Promise<number> {
  if (!isTauri) return mockSearchMediaCount(query);
  return tauriInvoke<number>("search_media_count", { query });
}

export async function createSmartAlbum(
  name: string,
  icon: string | null,
  rule: SmartAlbumRule,
): Promise<SmartAlbum> {
  if (!isTauri) return mockCreateSmartAlbum(name, icon, rule);
  return tauriInvoke<SmartAlbum>("create_smart_album", { name, icon, rule });
}

export async function listSmartAlbums(): Promise<SmartAlbum[]> {
  if (!isTauri) return mockListSmartAlbums();
  return tauriInvoke<SmartAlbum[]>("list_smart_albums");
}

export async function deleteSmartAlbum(id: number): Promise<void> {
  if (!isTauri) return mockDeleteSmartAlbum(id);
  return tauriInvoke("delete_smart_album", { id });
}

export async function getSmartAlbumMedia(
  id: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  if (!isTauri) return mockGetSmartAlbumMedia(id, offset, limit);
  return tauriInvoke<MediaItem[]>("get_smart_album_media", { id, offset, limit });
}

export async function generateMemories(): Promise<Memory[]> {
  if (!isTauri) return mockGenerateMemories();
  return tauriInvoke<Memory[]>("generate_memories");
}

export async function listMemories(): Promise<Memory[]> {
  if (!isTauri) return mockListMemories();
  return tauriInvoke<Memory[]>("list_memories");
}

export async function getMemoryMedia(
  memoryId: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  if (!isTauri) return mockGetMemoryMedia(memoryId, offset, limit);
  return tauriInvoke<MediaItem[]>("get_memory_media", { memoryId, offset, limit });
}

export async function getAiStatus(): Promise<AiStatus> {
  if (!isTauri) return mockGetAiStatus();
  return tauriInvoke<AiStatus>("get_ai_status");
}

export async function listPersons(): Promise<Person[]> {
  if (!isTauri) return mockListPersons();
  return tauriInvoke<Person[]>("list_persons");
}

export async function getPersonMedia(
  personId: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  if (!isTauri) return mockGetPersonMedia(personId, offset, limit);
  return tauriInvoke<MediaItem[]>("get_person_media", { personId, offset, limit });
}

export async function renamePerson(personId: number, name: string): Promise<void> {
  if (!isTauri) return mockRenamePerson(personId, name);
  return tauriInvoke("rename_person", { personId, name });
}
