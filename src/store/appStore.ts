import { useSyncExternalStore } from "react";
import type { MediaItem, ScanProgress, WatchedFolder } from "@/lib/tauri";

export type AppView =
  | "all"
  | "videos"
  | "timeline"
  | "locations"
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
  | "settings";

export type Theme = "light" | "dark" | "system";

export interface AppState {
  currentView: AppView;
  selectedAlbumId: number | null;
  selectedSmartAlbumId: number | null;
  selectedMemoryId: number | null;
  selectedPersonId: number | null;
  selectedMediaIds: number[];
  watchedFolders: WatchedFolder[];
  mediaItems: MediaItem[];
  totalCount: number;
  isScanning: boolean;
  scanProgress: ScanProgress | null;
  viewingMediaId: number | null;
  searchQuery: string;
  theme: Theme;
}

const initialState: AppState = {
  currentView: "all",
  selectedAlbumId: null,
  selectedSmartAlbumId: null,
  selectedMemoryId: null,
  selectedPersonId: null,
  selectedMediaIds: [],
  watchedFolders: [],
  mediaItems: [],
  totalCount: 0,
  isScanning: false,
  scanProgress: null,
  viewingMediaId: null,
  searchQuery: "",
  theme: "dark",
};

let state: AppState = { ...initialState };
const listeners = new Set<() => void>();

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
    selectedAlbumId: view === "album-detail" ? state.selectedAlbumId : null,
    selectedSmartAlbumId: view === "smart-album-detail" ? state.selectedSmartAlbumId : null,
    selectedMemoryId: view === "memory-detail" ? state.selectedMemoryId : null,
    selectedPersonId: view === "person-detail" ? state.selectedPersonId : null,
  });
}

export function openAlbumDetail(albumId: number) {
  setState({ currentView: "album-detail", selectedAlbumId: albumId });
}

export function closeAlbumDetail() {
  setState({ currentView: "albums", selectedAlbumId: null });
}

export function openSmartAlbumDetail(smartAlbumId: number) {
  setState({ currentView: "smart-album-detail", selectedSmartAlbumId: smartAlbumId });
}

export function closeSmartAlbumDetail() {
  setState({ currentView: "smart-albums", selectedSmartAlbumId: null });
}

export function openMemoryDetail(memoryId: number) {
  setState({ currentView: "memory-detail", selectedMemoryId: memoryId });
}

export function closeMemoryDetail() {
  setState({ currentView: "memories", selectedMemoryId: null });
}

export function openPersonDetail(personId: number) {
  setState({ currentView: "person-detail", selectedPersonId: personId });
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

export function setMedia(items: MediaItem[], totalCount: number) {
  setState({ mediaItems: items, totalCount });
}

export function appendMedia(items: MediaItem[]) {
  if (items.length === 0) return;
  setState({ mediaItems: [...state.mediaItems, ...items] });
}

export function setScanning(isScanning: boolean, progress: ScanProgress | null = null) {
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

export function selectMediaRange(fromId: number, toId: number) {
  const items = state.mediaItems;
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

export function setSearchQuery(query: string) {
  setState({ searchQuery: query });
}

export function useAppStore(): AppState {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
