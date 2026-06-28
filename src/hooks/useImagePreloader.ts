import { useEffect } from "react";
import {
  getMediaById,
  getMediaNeighbors,
  getOriginalUrl,
  getThumbnailUrl,
  type MediaItem,
} from "@/lib/tauri";

const ADJACENT_COUNT = 2;
const MAX_CONCURRENT = 2;

export async function getAdjacentMediaIds(
  mediaId: number,
  count: number,
): Promise<number[]> {
  const ids: number[] = [];

  let currentId = mediaId;
  for (let i = 0; i < count; i++) {
    const nb = await getMediaNeighbors(currentId);
    if (nb.prev_id == null) break;
    ids.push(nb.prev_id);
    currentId = nb.prev_id;
  }

  currentId = mediaId;
  for (let i = 0; i < count; i++) {
    const nb = await getMediaNeighbors(currentId);
    if (nb.next_id == null) break;
    ids.push(nb.next_id);
    currentId = nb.next_id;
  }

  return ids;
}

class ImagePreloadQueue {
  private running = 0;
  private queue: Array<{ url: string; resolve: () => void }> = [];
  private images: HTMLImageElement[] = [];
  private cancelled = false;

  constructor(private maxConcurrent: number) {}

  enqueue(url: string): Promise<void> {
    if (this.cancelled) return Promise.resolve();
    return new Promise((resolve) => {
      this.queue.push({ url, resolve });
      this.pump();
    });
  }

  private pump() {
    while (!this.cancelled && this.running < this.maxConcurrent && this.queue.length > 0) {
      const task = this.queue.shift()!;
      this.running++;
      const img = new Image();
      this.images.push(img);
      const done = () => {
        this.running--;
        task.resolve();
        if (!this.cancelled) this.pump();
      };
      img.onload = done;
      img.onerror = done;
      img.src = task.url;
    }
  }

  cancel() {
    this.cancelled = true;
    this.queue.length = 0;
    for (const img of this.images) {
      img.onload = null;
      img.onerror = null;
      img.src = "";
    }
    this.images.length = 0;
  }
}

export interface UseImagePreloaderOptions {
  mediaId: number;
  filmstrip: MediaItem[];
  currentImageLoaded: boolean;
  enabled: boolean;
}

export function useImagePreloader({
  mediaId,
  filmstrip,
  currentImageLoaded,
  enabled,
}: UseImagePreloaderOptions) {
  useEffect(() => {
    if (!enabled || !currentImageLoaded) return;

    let cancelled = false;
    const queue = new ImagePreloadQueue(MAX_CONCURRENT);

    void (async () => {
      const adjacentIds = await getAdjacentMediaIds(mediaId, ADJACENT_COUNT);
      if (cancelled) return;

      const adjacentItems = await Promise.all(adjacentIds.map((id) => getMediaById(id)));
      if (cancelled) return;

      const originalUrls = adjacentItems
        .filter((item): item is MediaItem => item != null && item.media_type !== "Video")
        .map((item) => getOriginalUrl(item.path));

      const thumbUrls = filmstrip
        .filter((item) => item.id !== mediaId && item.media_type !== "Video")
        .map((item) => getThumbnailUrl(item.id, "small"));

      const urls = [...originalUrls, ...thumbUrls];

      await Promise.all(urls.map((url) => (cancelled ? Promise.resolve() : queue.enqueue(url))));
    })();

    return () => {
      cancelled = true;
      queue.cancel();
    };
  }, [mediaId, filmstrip, currentImageLoaded, enabled]);
}
