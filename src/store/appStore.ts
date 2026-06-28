import { useSyncExternalStore } from "react";
import type { MediaItem, ScanProgress, WatchedFolder } from "@/lib/tauri";

export type AppView =
  | "all"
  | "timeline"
  | "locations"
  | "people"
  | "duplicates"
  | "screenshots"
  | "settings";

export interface AppState {
  currentView: AppView;
  selectedMediaIds: number[];
  watchedFolders: WatchedFolder[];
  mediaItems: MediaItem[];
  totalCount: number;
  isScanning: boolean;
  scanProgress: ScanProgress | null;
  viewingMediaId: number | null;
}

const initialState: AppState = {
  currentView: "all",
  selectedMediaIds: [],
  watchedFolders: [],
  mediaItems: [],
  totalCount: 0,
  isScanning: false,
  scanProgress: null,
  viewingMediaId: null,
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
  setState({ currentView: view });
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

export function clearMediaSelection() {
  setState({ selectedMediaIds: [] });
}

export function openViewer(id: number) {
  setState({ viewingMediaId: id });
}

export function closeViewer() {
  setState({ viewingMediaId: null });
}

export function useAppStore(): AppState {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
