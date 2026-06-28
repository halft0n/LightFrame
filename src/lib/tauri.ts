import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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
}

export interface ScanProgress {
  folder_id: number;
  scanned: number;
  total: number;
  status: ScanStatus;
}

export function getThumbnailUrl(id: number, size: "small" | "large" | "micro" = "small"): string {
  return `thumb://localhost/${id}/${size}`;
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

export async function getMediaCount(): Promise<number> {
  return invoke<number>("get_media_count");
}

export async function scanFolder(folderId: number): Promise<void> {
  return invoke("scan_folder", { folderId });
}

export async function onScanProgress(
  callback: (progress: ScanProgress) => void,
): Promise<UnlistenFn> {
  return listen<ScanProgress>("scan-progress", (event) => {
    callback(event.payload);
  });
}
