import { useSyncExternalStore, useMemo } from "react";
import type { MediaItem, ScanProgress, WatchedFolder } from "@/lib/tauri";
import { getMediaCount, getMediaPage } from "@/lib/tauri";

export type MediaCursor = [string, number] | null;

const MEDIA_PAGE_SIZE = 60;
const MAX_MEDIA_ITEMS = 5000;
const MEDIA_WINDOW_HALF = MAX_MEDIA_ITEMS / 2;

export type AppView =
  | "all"
  | "videos"
  | "timeline"
  | "locations"
  | "map"
  | "people"
  | "person-detail"
  | "duplicates"
  | "screenshots"
  | "albums"
  | "album-detail"
  | "smart-albums"
  | "smart-album-detail"
  | "memories"
  | "memory-detail"
  | "favorites"
  | "deleted"
  | "folder"
  | "settings";

export type Theme = "light" | "dark" | "system";

export type ThumbnailSize = "small" | "medium" | "large";

export type SlideshowSpeed = 3 | 5 | 10;

export const THUMBNAIL_WIDTHS: Record<ThumbnailSize, number> = {
  small: 120,
  medium: 180,
  large: 260,
};

export type SearchMode = "text" | "semantic";

export interface AppState {
  currentView: AppView;
  selectedAlbumId: number | null;
  selectedSmartAlbumId: number | null;
  selectedMemoryId: number | null;
  selectedPersonId: number | null;
  selectedFolderId: number | null;
  selectedFolderPath: string | null;
  selectedMediaIds: number[];
  watchedFolders: WatchedFolder[];
  mediaItems: MediaItem[];
  totalCount: number;
  mediaCursor: MediaCursor;
  mediaScrollIndex: number;
  isScanning: boolean;
  scanProgress: ScanProgress | null;
  viewingMediaId: number | null;
  searchQuery: string;
  searchMode: SearchMode;
  searchHistory: string[];
  thumbnailSize: ThumbnailSize;
  theme: Theme;
  slideshowActive: boolean;
  slideshowMediaIds: number[];
  slideshowIndex: number;
  slideshowSpeed: SlideshowSpeed;
}

const initialState: AppState = {
  currentView: "all",
  selectedAlbumId: null,
  selectedSmartAlbumId: null,
  selectedMemoryId: null,
  selectedPersonId: null,
  selectedFolderId: null,
  selectedFolderPath: null,
  selectedMediaIds: [],
  watchedFolders: [],
  mediaItems: [],
  totalCount: 0,
  mediaCursor: null,
  mediaScrollIndex: 0,
  isScanning: false,
  scanProgress: null,
  viewingMediaId: null,
  searchQuery: "",
  searchMode: "text",
  searchHistory: [],
  thumbnailSize: "medium",
  theme: "dark",
  slideshowActive: false,
  slideshowMediaIds: [],
  slideshowIndex: 0,
  slideshowSpeed: 5,
};

let state: AppState = { ...initialState };
const listeners = new Set<() => void>();

export function resetStore() {
  state = { ...initialState };
  emit();
}

function emit() {
  for (const listener of listeners) {
    listener();
  }
}

function setState(partial: Partial<AppState>) {
  state = { ...state, ...partial };
  emit();
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getSnapshot(): AppState {
  return state;
}

export function setView(view: AppView) {
  setState({
    currentView: view,
    viewingMediaId: null,
    slideshowActive: false,
    slideshowMediaIds: [],
    slideshowIndex: 0,
    selectedAlbumId: view === "album-detail" ? state.selectedAlbumId : null,
    selectedSmartAlbumId:
      view === "smart-album-detail" ? state.selectedSmartAlbumId : null,
    selectedMemoryId: view === "memory-detail" ? state.selectedMemoryId : null,
    selectedPersonId: view === "person-detail" ? state.selectedPersonId : null,
    selectedFolderId: view === "folder" ? state.selectedFolderId : null,
    selectedFolderPath: view === "folder" ? state.selectedFolderPath : null,
  });
}

export interface NavigateParams {
  folderId?: number;
  folderPath?: string;
}

export function navigate(view: AppView, params?: NavigateParams) {
  if (view === "folder") {
    setState({
      currentView: "folder",
      viewingMediaId: null,
      slideshowActive: false,
      slideshowMediaIds: [],
      slideshowIndex: 0,
      selectedFolderId: params?.folderId ?? null,
      selectedFolderPath: params?.folderPath ?? null,
      selectedAlbumId: null,
      selectedSmartAlbumId: null,
      selectedMemoryId: null,
      selectedPersonId: null,
    });
    return;
  }
  setView(view);
}

export function openAlbumDetail(albumId: number) {
  setState({
    currentView: "album-detail",
    selectedAlbumId: albumId,
    viewingMediaId: null,
    slideshowActive: false,
  });
}

export function closeAlbumDetail() {
  setState({ currentView: "albums", selectedAlbumId: null });
}

export function openSmartAlbumDetail(smartAlbumId: number) {
  setState({
    currentView: "smart-album-detail",
    selectedSmartAlbumId: smartAlbumId,
    viewingMediaId: null,
    slideshowActive: false,
  });
}

export function closeSmartAlbumDetail() {
  setState({ currentView: "smart-albums", selectedSmartAlbumId: null });
}

export function openMemoryDetail(memoryId: number) {
  setState({
    currentView: "memory-detail",
    selectedMemoryId: memoryId,
    viewingMediaId: null,
    slideshowActive: false,
  });
}

export function closeMemoryDetail() {
  setState({ currentView: "memories", selectedMemoryId: null });
}

export function openPersonDetail(personId: number) {
  setState({
    currentView: "person-detail",
    selectedPersonId: personId,
    viewingMediaId: null,
    slideshowActive: false,
  });
}

export function closePersonDetail() {
  setState({ currentView: "people", selectedPersonId: null });
}

export function setWatchedFolders(folders: WatchedFolder[]) {
  setState({ watchedFolders: folders });
}

export function addFolder(folder: WatchedFolder) {
  setState({ watchedFolders: [...state.watchedFolders, folder] });
}

export function removeFolder(id: number) {
  setState({
    watchedFolders: state.watchedFolders.filter((f) => f.id !== id),
  });
}

export function updateFolder(id: number, update: Partial<WatchedFolder>) {
  setState({
    watchedFolders: state.watchedFolders.map((f) =>
      f.id === id ? { ...f, ...update } : f,
    ),
  });
}

function trimMediaItems(items: MediaItem[], scrollIndex: number): MediaItem[] {
  if (items.length <= MAX_MEDIA_ITEMS) return items;
  const start = Math.max(0, scrollIndex - MEDIA_WINDOW_HALF);
  const end = Math.min(items.length, start + MAX_MEDIA_ITEMS);
  const adjustedStart = Math.max(0, end - MAX_MEDIA_ITEMS);
  return items.slice(adjustedStart, end);
}

export function setMediaScrollIndex(index: number) {
  const clamped = Math.max(0, index);
  if (clamped === state.mediaScrollIndex) return;
  const mediaItems = trimMediaItems(state.mediaItems, clamped);
  setState({
    mediaScrollIndex: clamped,
    mediaItems,
    mediaCursor: mediaCursorFromItems(mediaItems),
  });
}

export function setMedia(items: MediaItem[], totalCount: number) {
  const mediaItems = trimMediaItems(items, state.mediaScrollIndex);
  setState({
    mediaItems,
    totalCount,
    mediaCursor: mediaCursorFromItems(mediaItems),
  });
}

export function appendMedia(items: MediaItem[]) {
  if (items.length === 0) return;
  const combined = [...state.mediaItems, ...items];
  const mediaItems = trimMediaItems(combined, state.mediaScrollIndex);
  setState({ mediaItems, mediaCursor: mediaCursorFromItems(mediaItems) });
}

export function mergeNewMedia(items: MediaItem[]) {
  if (items.length === 0) return;
  const existingIds = new Set(state.mediaItems.map((m) => m.id));
  const newItems = items.filter((m) => !existingIds.has(m.id));
  if (newItems.length === 0) return;
  const combined = [...newItems, ...state.mediaItems].sort((a, b) => {
    const dateA = a.created_at ?? a.modified_at;
    const dateB = b.created_at ?? b.modified_at;
    if (dateA !== dateB) return dateB.localeCompare(dateA);
    return b.id - a.id;
  });
  const mediaItems = trimMediaItems(combined, state.mediaScrollIndex);
  setState({
    mediaItems,
    totalCount: state.totalCount + newItems.length,
    mediaCursor: mediaCursorFromItems(mediaItems),
  });
}

function mediaCursorFromItems(items: MediaItem[]): MediaCursor {
  if (items.length === 0) return null;
  const last = items[items.length - 1];
  if (!last.created_at) return null;
  return [last.created_at, last.id];
}

export async function loadMedia() {
  try {
    const [items, totalCount] = await Promise.all([
      getMediaPage(MEDIA_PAGE_SIZE),
      getMediaCount(),
    ]);
    setMedia(items, totalCount);
  } catch (error) {
    console.error("Failed to load media:", error);
  }
}

export async function loadMoreMedia() {
  const { mediaItems, totalCount, mediaCursor } = state;
  if (mediaItems.length >= totalCount) return;

  try {
    const items = await getMediaPage(MEDIA_PAGE_SIZE, mediaCursor ?? undefined);
    appendMedia(items);
  } catch (error) {
    console.error("Failed to load more media:", error);
    throw error;
  }
}

export function setScanning(
  isScanning: boolean,
  progress: ScanProgress | null = null,
) {
  setState({ isScanning, scanProgress: progress });
}

export function toggleMediaSelection(id: number) {
  const selected = new Set(state.selectedMediaIds);
  if (selected.has(id)) {
    selected.delete(id);
  } else {
    selected.add(id);
  }
  setState({ selectedMediaIds: [...selected] });
}

export function setSingleMediaSelection(id: number) {
  setState({ selectedMediaIds: [id] });
}

export function selectMediaRange(
  fromId: number,
  toId: number,
  contextItems?: { id: number }[],
) {
  const items = contextItems ?? state.mediaItems;
  const fromIdx = items.findIndex((m) => m.id === fromId);
  const toIdx = items.findIndex((m) => m.id === toId);
  if (fromIdx === -1 || toIdx === -1) return;
  const start = Math.min(fromIdx, toIdx);
  const end = Math.max(fromIdx, toIdx);
  const ids = items.slice(start, end + 1).map((m) => m.id);
  setState({ selectedMediaIds: ids });
}

export function setMediaSelection(ids: number[]) {
  setState({ selectedMediaIds: ids });
}

export function clearMediaSelection() {
  setState({ selectedMediaIds: [] });
}

export function setTheme(theme: Theme) {
  setState({ theme });
}

export function openViewer(id: number) {
  setState({ viewingMediaId: id });
}

export function closeViewer() {
  setState({ viewingMediaId: null });
}

export function startSlideshow(mediaIds: number[], startAtId?: number) {
  if (mediaIds.length === 0) return;
  const startIndex =
    startAtId != null ? Math.max(0, mediaIds.indexOf(startAtId)) : 0;
  setState({
    slideshowActive: true,
    slideshowMediaIds: mediaIds,
    slideshowIndex: startIndex === -1 ? 0 : startIndex,
    viewingMediaId: null,
  });
}

export function closeSlideshow() {
  setState({
    slideshowActive: false,
    slideshowMediaIds: [],
    slideshowIndex: 0,
  });
}

export function setSlideshowIndex(index: number) {
  const { slideshowMediaIds } = state;
  if (slideshowMediaIds.length === 0) return;
  const clamped = Math.max(0, Math.min(index, slideshowMediaIds.length - 1));
  setState({ slideshowIndex: clamped });
}

export function nextSlideshow() {
  const { slideshowMediaIds, slideshowIndex } = state;
  if (slideshowMediaIds.length === 0) return;
  const next =
    slideshowIndex >= slideshowMediaIds.length - 1 ? 0 : slideshowIndex + 1;
  setState({ slideshowIndex: next });
}

export function prevSlideshow() {
  const { slideshowMediaIds, slideshowIndex } = state;
  if (slideshowMediaIds.length === 0) return;
  const prev =
    slideshowIndex <= 0 ? slideshowMediaIds.length - 1 : slideshowIndex - 1;
  setState({ slideshowIndex: prev });
}

export function setSlideshowSpeed(speed: SlideshowSpeed) {
  setState({ slideshowSpeed: speed });
}

export function setSearchQuery(query: string) {
  setState({ searchQuery: query });
}

export function setSearchMode(mode: SearchMode) {
  setState({ searchMode: mode });
}

export function addSearchHistory(query: string) {
  const trimmed = query.trim();
  if (!trimmed) return;
  const filtered = state.searchHistory.filter((q) => q !== trimmed);
  setState({ searchHistory: [trimmed, ...filtered].slice(0, 10) });
}

export function clearSearchHistory() {
  setState({ searchHistory: [] });
}

export function setThumbnailSize(size: ThumbnailSize) {
  setState({ thumbnailSize: size });
}

export function useAppStore(): AppState {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export function useAppStoreSelector<T>(selector: (state: AppState) => T): T {
  const state = useAppStore();
  return useMemo(() => selector(state), [state, selector]);
}
