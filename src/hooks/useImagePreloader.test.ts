import { renderHook, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { getAdjacentMediaIds, useImagePreloader } from "./useImagePreloader";
import type { MediaItem } from "@/lib/tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockPhoto = (id: number, path = `/photos/${id}.jpg`): MediaItem => ({
  id,
  path,
  filename: `${id}.jpg`,
  media_type: "Photo",
  size_bytes: 1024,
  modified_at: "2024-06-15T10:00:00",
});

type MockImage = {
  src: string;
  onload: (() => void) | null;
  onerror: (() => void) | null;
};

describe("getAdjacentMediaIds", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("walks prev and next chains up to count", async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation(
      (cmd: string, args?: Record<string, unknown>) => {
        if (cmd !== "get_media_neighbors") return Promise.resolve(null);
        const id = args?.id as number;
        const map: Record<
          number,
          { prev_id: number | null; next_id: number | null }
        > = {
          5: { prev_id: 4, next_id: 6 },
          4: { prev_id: 3, next_id: null },
          3: { prev_id: null, next_id: null },
          6: { prev_id: null, next_id: 7 },
          7: { prev_id: null, next_id: null },
        };
        return Promise.resolve(map[id] ?? { prev_id: null, next_id: null });
      },
    );

    const ids = await getAdjacentMediaIds(5, 2);
    expect(ids).toEqual([4, 3, 6, 7]);
  });
});

describe("useImagePreloader", () => {
  const createdImages: MockImage[] = [];
  let OriginalImage: typeof Image;

  beforeEach(() => {
    vi.clearAllMocks();
    createdImages.length = 0;
    OriginalImage = globalThis.Image;

    class MockImageClass {
      src = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;

      constructor() {
        createdImages.push(this);
      }
    }

    vi.stubGlobal("Image", MockImageClass);

    (invoke as ReturnType<typeof vi.fn>).mockImplementation(
      (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === "get_media_neighbors") {
          const id = args?.id as number;
          if (id === 1) return Promise.resolve({ prev_id: 10, next_id: 20 });
          if (id === 10) return Promise.resolve({ prev_id: 11, next_id: null });
          if (id === 20) return Promise.resolve({ prev_id: null, next_id: 21 });
          if (id === 11)
            return Promise.resolve({ prev_id: null, next_id: null });
          if (id === 21)
            return Promise.resolve({ prev_id: null, next_id: null });
          return Promise.resolve({ prev_id: null, next_id: null });
        }
        if (cmd === "get_media_by_id") {
          const id = args?.id as number;
          return Promise.resolve(mockPhoto(id));
        }
        return Promise.resolve(null);
      },
    );
  });

  afterEach(() => {
    vi.stubGlobal("Image", OriginalImage);
  });

  function flushAllImageLoads(maxRounds = 10) {
    for (let round = 0; round < maxRounds; round++) {
      const pending = createdImages.filter((img) => img.src !== "");
      if (pending.length === 0) break;
      for (const img of pending) {
        img.onload?.();
      }
    }
  }

  it("does not preload until current image is loaded", () => {
    renderHook(() =>
      useImagePreloader({
        mediaId: 1,
        filmstrip: [mockPhoto(1), mockPhoto(3)],
        currentImageLoaded: false,
        enabled: true,
      }),
    );

    expect(invoke).not.toHaveBeenCalledWith(
      "get_media_by_id",
      expect.anything(),
    );
    expect(createdImages).toHaveLength(0);
  });

  it("preloads adjacent originals and filmstrip thumbnails after load", async () => {
    renderHook(() =>
      useImagePreloader({
        mediaId: 1,
        filmstrip: [mockPhoto(1), mockPhoto(3)],
        currentImageLoaded: true,
        enabled: true,
      }),
    );

    await waitFor(() => {
      expect(createdImages.length).toBeGreaterThan(0);
    });

    flushAllImageLoads();
    await waitFor(() => {
      expect(
        createdImages.some((img) => img.src.startsWith("original://")),
      ).toBe(true);
      expect(
        createdImages.some((img) => img.src === "thumb://localhost/3/small"),
      ).toBe(true);
    });
  });

  it("cancels in-flight preloads when mediaId changes", async () => {
    const { rerender } = renderHook(
      (props: { mediaId: number; loaded: boolean }) =>
        useImagePreloader({
          mediaId: props.mediaId,
          filmstrip: [mockPhoto(props.mediaId), mockPhoto(3)],
          currentImageLoaded: props.loaded,
          enabled: true,
        }),
      { initialProps: { mediaId: 1, loaded: true } },
    );

    await waitFor(() => {
      expect(createdImages.length).toBeGreaterThan(0);
    });

    const staleImages = [...createdImages];
    rerender({ mediaId: 99, loaded: true });

    await waitFor(() => {
      for (const img of staleImages) {
        expect(img.src).toBe("");
      }
    });
  });

  it("cleans up preloads on unmount", async () => {
    const { unmount } = renderHook(() =>
      useImagePreloader({
        mediaId: 1,
        filmstrip: [mockPhoto(1), mockPhoto(3)],
        currentImageLoaded: true,
        enabled: true,
      }),
    );

    await waitFor(() => {
      expect(createdImages.length).toBeGreaterThan(0);
    });

    const loaded = createdImages.filter((img) => img.src !== "");
    unmount();

    for (const img of loaded) {
      expect(img.src).toBe("");
      expect(img.onload).toBeNull();
      expect(img.onerror).toBeNull();
    }
  });
});
