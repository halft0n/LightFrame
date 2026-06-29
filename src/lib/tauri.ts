import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t } from "@/i18n";
import { localizeError } from "./errors";

export async function invokeCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return args !== undefined
      ? await invoke<T>(cmd, args)
      : await invoke<T>(cmd);
  } catch (error) {
    throw new Error(localizeError(error, t));
  }
}

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
  dhash?: number | null;
  phash?: number | null;
  latitude?: number | null;
  longitude?: number | null;
  camera_make?: string | null;
  camera_model?: string | null;
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

export interface ModelStatus {
  models_dir: string;
  clip_available: boolean;
  face_available: boolean;
  models: ModelFileStatus[];
}

export interface ModelFileStatus {
  name: string;
  filename: string;
  url: string;
  size_mb: number;
  description: string;
  installed: boolean;
  file_size_bytes: number | null;
  sha256_verified: boolean | null;
}

export type ScreenshotCategory =
  | "all"
  | "generic"
  | "code"
  | "chat"
  | "document"
  | "game"
  | "webpage";

export interface Person {
  id: number;
  name: string | null;
  face_count: number;
  cover_face_id: number | null;
  sample_media_ids: number[];
  created_at: string;
}

export interface SimilarPhoto {
  media_id: number;
  similarity: number;
  file_name: string;
  file_path: string;
}

export interface FaceInfo {
  id: number;
  media_id: number;
  bbox: [number, number, number, number];
  confidence: number;
  person_id: number | null;
}

export interface FaceDetectionProgress {
  processed: number;
  total: number;
  faces_found: number;
  status: "detecting" | "complete" | "error";
}

export interface FaceDetectionBatchResult {
  media_processed: number;
  faces_found: number;
}

export interface SearchResult {
  media_id: number;
  file_name: string;
  file_path: string;
  relevance: number;
}

export interface SemanticSearchResponse {
  results: SearchResult[];
  used_semantic: boolean;
}

export interface ModelDownloadProgress {
  filename: string;
  downloaded: number;
  total: number;
}

export interface PersonClusterInfo {
  person_id: number;
  name: string | null;
  face_count: number;
  avg_intra_cluster_distance: number;
}

export function getFaceThumbnailUrl(faceId: number): string {
  return `face://localhost/${faceId}`;
}

export function getThumbnailUrl(id: number, size: "small" | "large" | "micro" = "small"): string {
  return `thumb://localhost/${id}/${size}`;
}

export function getOriginalUrl(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  return `original://localhost/${encodeURIComponent(normalized)}`;
}

export async function addWatchedFolder(path: string): Promise<WatchedFolder> {
  return invoke<WatchedFolder>("add_watched_folder", { path });
}

export async function removeWatchedFolder(id: number): Promise<void> {
  return invoke("remove_watched_folder", { id });
}

export async function listWatchedFolders(): Promise<WatchedFolder[]> {
  return invoke<WatchedFolder[]>("list_watched_folders");
}

export async function getMediaList(offset: number, limit: number): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_media_list", { offset, limit });
}

export async function getMediaPage(limit: number, cursor?: [string, number]) {
  return invokeCommand<MediaItem[]>("get_media_page", { limit, cursor: cursor ?? null });
}

export async function getMediaCount(): Promise<number> {
  return invokeCommand<number>("get_media_count");
}

export async function getMediaByFolder(
  folderId: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_media_by_folder", { folderId, offset, limit });
}

export async function getMediaCountByFolder(folderId: number): Promise<number> {
  return invoke<number>("get_media_count_by_folder", { folderId });
}

export async function batchExport(
  mediaIds: number[],
  outputDir: string,
): Promise<number> {
  return invoke<number>("batch_export", { mediaIds, outputDir });
}

export async function getMediaByType(
  mediaType: MediaType,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_media_by_type", {
    mediaType,
    offset,
    limit,
  });
}

export async function getMediaCountByType(mediaType: MediaType): Promise<number> {
  return invoke<number>("get_media_count_by_type", { mediaType });
}

export async function getMediaById(id: number): Promise<MediaItem | null> {
  return invoke<MediaItem | null>("get_media_by_id", { id });
}

export type TimelineCursor = { createdAt: string; id: number } | null;

export async function getTimelineGroups(
  limit = 200,
  cursor?: TimelineCursor,
): Promise<TimelineGroup[]> {
  return invoke<TimelineGroup[]>("get_timeline_groups", {
    limit,
    cursorCreatedAt: cursor?.createdAt ?? null,
    cursorId: cursor?.id ?? null,
  });
}

export interface BatchEmbedResult {
  processed: number;
  succeeded: number;
  failed: number;
  errors: string[];
}

export async function computeClipEmbeddingsBatch(limit = 32): Promise<BatchEmbedResult> {
  return invoke<BatchEmbedResult>("compute_clip_embeddings_batch", { limit });
}

export async function getMediaNeighbors(id: number): Promise<MediaNeighbors> {
  return invoke<MediaNeighbors>("get_media_neighbors", { id });
}

export async function getMediaWindow(mediaId: number, radius: number): Promise<MediaItem[]> {
  return invokeCommand<MediaItem[]>("get_media_window", { mediaId, radius });
}

export async function scanFolder(folderId: number): Promise<void> {
  return invoke("scan_folder", { folderId });
}

export async function onScanProgress(
  callback: (progress: ScanProgress) => void,
): Promise<() => void> {
  return listen<ScanProgress>("scan-progress", (event) => {
    callback(event.payload);
  });
}

export async function onFolderChanged(
  callback: (folderId: number) => void,
): Promise<() => void> {
  return listen<{ folder_id: number }>("folder-changed", (event) => {
    callback(event.payload.folder_id);
  });
}

export async function runDedupScan(): Promise<DedupScanResult> {
  return invoke<DedupScanResult>("run_dedup_scan");
}

export async function getDuplicateGroups(): Promise<DuplicateGroup[]> {
  return invoke<DuplicateGroup[]>("get_duplicate_groups");
}

export async function getDuplicateCount(): Promise<number> {
  return invoke<number>("get_duplicate_count");
}

export async function resolveDuplicate(
  groupId: number,
  keepMediaId: number,
  deleteFiles: boolean,
): Promise<void> {
  return invoke("resolve_duplicate", {
    groupId,
    keepMediaId,
    deleteFiles,
  });
}

export async function dismissDuplicateGroup(groupId: number): Promise<void> {
  return invoke("dismiss_duplicate_group", { groupId });
}

export async function getLocationGroups(): Promise<LocationGroup[]> {
  return invoke<LocationGroup[]>("get_location_groups");
}

export async function getMediaByLocation(
  country: string,
  city: string | null,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_media_by_location", {
    country,
    city,
    offset,
    limit,
  });
}

export async function getLocationStats(): Promise<LocationStats> {
  return invoke<LocationStats>("get_location_stats");
}

export async function createAlbum(
  name: string,
  description?: string | null,
): Promise<Album> {
  return invokeCommand<Album>("create_album", { name, description: description ?? null });
}

export async function deleteAlbum(id: number): Promise<void> {
  return invokeCommand("delete_album", { id });
}

export async function updateAlbum(
  id: number,
  name: string,
  description?: string | null,
): Promise<void> {
  return invokeCommand("update_album", { id, name, description: description ?? null });
}

export async function setAlbumCover(albumId: number, mediaId: number): Promise<void> {
  return invoke("set_album_cover", { albumId, mediaId });
}

export async function listAlbums(): Promise<Album[]> {
  return invoke<Album[]>("list_albums");
}

export async function addToAlbum(albumId: number, mediaIds: number[]): Promise<void> {
  return invokeCommand("add_to_album", { albumId, mediaIds });
}

export async function removeFromAlbum(albumId: number, mediaId: number): Promise<void> {
  return invokeCommand("remove_from_album", { albumId, mediaId });
}

export async function getAlbumMedia(
  albumId: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_album_media", { albumId, offset, limit });
}

export async function toggleFavorite(mediaId: number): Promise<boolean> {
  return invokeCommand<boolean>("toggle_favorite", { mediaId });
}

export async function getFavoriteState(mediaId: number): Promise<boolean> {
  try {
    return await invoke<boolean>("is_favorite", { mediaId });
  } catch (error) {
    console.error("getFavoriteState failed:", error);
    return false;
  }
}

export async function getFavorites(offset: number, limit: number): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_favorites", { offset, limit });
}

export async function getFavoritesCount(): Promise<number> {
  return invoke<number>("get_favorites_count");
}

export async function deleteMedia(mediaId: number): Promise<void> {
  return invokeCommand("delete_media", { mediaId });
}

export async function getDeletedMedia(): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_deleted_media");
}

export async function restoreMedia(mediaId: number): Promise<void> {
  return invoke("restore_media", { mediaId });
}

export async function permanentlyDelete(mediaId: number): Promise<void> {
  return invoke("permanently_delete", { mediaId });
}

export async function batchDeleteMedia(mediaIds: number[]): Promise<number> {
  return invokeCommand<number>("batch_delete_media", { mediaIds });
}

export async function batchAddToAlbum(albumId: number, mediaIds: number[]): Promise<void> {
  return invokeCommand("batch_add_to_album", { albumId, mediaIds });
}

export async function batchToggleFavorite(
  mediaIds: number[],
  favorite: boolean,
): Promise<number> {
  return invokeCommand<number>("batch_toggle_favorite", { mediaIds, favorite });
}

export async function batchRestoreMedia(mediaIds: number[]): Promise<number> {
  return invoke<number>("batch_restore_media", { mediaIds });
}

export async function batchPermanentDelete(mediaIds: number[]): Promise<number> {
  return invoke<number>("batch_permanent_delete", { mediaIds });
}

export async function searchMedia(
  query: string,
  limit: number,
  offset: number,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("search_media", { query, limit, offset });
}

export async function searchMediaCount(query: string): Promise<number> {
  return invoke<number>("search_media_count", { query });
}

export async function semanticSearch(
  query: string,
  limit?: number,
): Promise<SemanticSearchResponse> {
  return invoke<SemanticSearchResponse>("semantic_search", {
    queryText: query,
    limit: limit ?? 50,
  });
}

export async function createSmartAlbum(
  name: string,
  icon: string | null,
  rule: SmartAlbumRule,
): Promise<SmartAlbum> {
  return invoke<SmartAlbum>("create_smart_album", { name, icon, rule });
}

export async function listSmartAlbums(): Promise<SmartAlbum[]> {
  return invoke<SmartAlbum[]>("list_smart_albums");
}

export async function deleteSmartAlbum(id: number): Promise<void> {
  return invoke("delete_smart_album", { id });
}

export async function getSmartAlbumMedia(
  id: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_smart_album_media", { id, offset, limit });
}

export async function generateMemories(): Promise<Memory[]> {
  return invoke<Memory[]>("generate_memories");
}

export async function getOnThisDay(limit = 20): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_on_this_day", { limit });
}

export async function listMemories(): Promise<Memory[]> {
  return invoke<Memory[]>("list_memories");
}

export async function getMemoryMedia(
  memoryId: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_memory_media", { memoryId, offset, limit });
}

export async function getAiStatus(): Promise<AiStatus> {
  return invoke<AiStatus>("get_ai_status");
}

export async function getModelStatus(): Promise<ModelStatus> {
  return invoke<ModelStatus>("get_model_status");
}

export async function downloadModel(filename: string): Promise<string> {
  return invoke<string>("download_model", { filename });
}

export async function openModelsDir(): Promise<void> {
  return invoke("open_models_dir");
}

export async function getScreenshots(
  category: ScreenshotCategory,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  const screenshotType = category === "all" ? null : category;
  return invoke<MediaItem[]>("get_screenshots", {
    screenshotType,
    limit,
    offset,
  });
}

export async function getScreenshotCount(category: ScreenshotCategory): Promise<number> {
  const screenshotType = category === "all" ? null : category;
  return invoke<number>("get_screenshot_count", { screenshotType });
}

export async function computeClipEmbedding(mediaId: number): Promise<void> {
  return invoke("compute_clip_embedding", { mediaId });
}

export async function findSimilarPhotos(
  mediaId: number,
  limit?: number,
): Promise<SimilarPhoto[]> {
  return invoke<SimilarPhoto[]>("find_similar_photos", { mediaId, limit: limit ?? 20 });
}

export async function detectFaces(mediaId: number): Promise<FaceInfo[]> {
  return invoke<FaceInfo[]>("detect_faces", { mediaId });
}

export async function detectFacesBatch(): Promise<FaceDetectionBatchResult> {
  return invoke<FaceDetectionBatchResult>("detect_faces_batch");
}

export async function onFaceDetectionProgress(
  callback: (progress: FaceDetectionProgress) => void,
): Promise<() => void> {
  return listen<FaceDetectionProgress>("face-detection-progress", (event) => {
    callback(event.payload);
  });
}

export async function getFaces(mediaId: number): Promise<FaceInfo[]> {
  return invoke<FaceInfo[]>("get_faces", { mediaId });
}

export async function getPersonFaces(
  personId: number,
  offset: number,
  limit: number,
): Promise<FaceInfo[]> {
  return invoke<FaceInfo[]>("get_person_faces", { personId, offset, limit });
}

export async function listPersons(): Promise<Person[]> {
  return invoke<Person[]>("list_persons");
}

export async function getPersonMedia(
  personId: number,
  offset: number,
  limit: number,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_person_media", { personId, offset, limit });
}

export async function renamePerson(personId: number, name: string): Promise<void> {
  return invoke("rename_person", { personId, name });
}

export async function clusterFaces(threshold?: number): Promise<PersonClusterInfo[]> {
  return invoke<PersonClusterInfo[]>("cluster_faces", { threshold: threshold ?? null });
}

export async function mergePersons(personIds: number[]): Promise<void> {
  if (personIds.length < 2) return;
  const targetId = personIds[0]!;
  for (const sourceId of personIds.slice(1)) {
    await invoke("merge_persons", { personIdA: targetId, personIdB: sourceId });
  }
}

export async function splitFaceFromPerson(
  faceId: number,
  newPersonName?: string,
): Promise<number> {
  return invoke<number>("split_face_from_person", {
    faceId,
    newPersonName: newPersonName ?? null,
  });
}

export async function saveEdit(mediaId: number, params: string): Promise<void> {
  return invoke("save_edit", { mediaId, params });
}

export async function getEdit(mediaId: number): Promise<string | null> {
  return invoke<string | null>("get_edit", { mediaId });
}

export async function revertEdit(mediaId: number): Promise<void> {
  return invoke("revert_edit", { mediaId });
}

export async function hasEdits(mediaId: number): Promise<boolean> {
  return invoke<boolean>("has_edits", { mediaId });
}

export async function exportEdited(
  mediaId: number,
  outputPath: string,
  quality = 92,
): Promise<void> {
  return invoke("export_edited", { mediaId, outputPath, quality });
}

export interface ThumbnailRegenProgress {
  processed: number;
  total: number;
  regenerated: number;
  status: string;
}

export interface ThumbnailRegenResult {
  regenerated: number;
}

export async function regenerateThumbnails(): Promise<ThumbnailRegenResult> {
  return invoke<ThumbnailRegenResult>("regenerate_thumbnails");
}

export async function regenerateThumbnailSingle(mediaId: number): Promise<boolean> {
  return invoke<boolean>("regenerate_thumbnail_single", { mediaId });
}

export async function onThumbnailRegenProgress(
  callback: (progress: ThumbnailRegenProgress) => void,
): Promise<() => void> {
  return listen<ThumbnailRegenProgress>("thumbnail-regen-progress", (event) => {
    callback(event.payload);
  });
}

export interface GeoCluster {
  latitude: number;
  longitude: number;
  count: number;
  media_ids: number[];
}

export async function getMediaWithGeo(
  limit = 5000,
  offset = 0,
): Promise<MediaItem[]> {
  return invoke<MediaItem[]>("get_media_with_geo", { limit, offset });
}

export async function getGeoClusters(gridSize = 0.5): Promise<GeoCluster[]> {
  return invoke<GeoCluster[]>("get_geo_clusters", { gridSize });
}
