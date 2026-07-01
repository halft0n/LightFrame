import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PhotoCard } from "./PhotoCard";
import type { MediaItem } from "@/lib/tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn(
    (filePath: string, protocol: string = "asset") =>
      `${protocol}://localhost/${filePath}`,
  ),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const photoItem: MediaItem = {
  id: 1,
  path: "/photos/sunset.jpg",
  filename: "sunset.jpg",
  media_type: "Photo",
  size_bytes: 2048,
  modified_at: "2024-01-01T00:00:00",
};

const videoItem: MediaItem = {
  id: 2,
  path: "/videos/clip.mp4",
  filename: "clip.mp4",
  media_type: "Video",
  size_bytes: 4096,
  modified_at: "2024-01-01T00:00:00",
  duration_sec: 125,
};

const rawItem: MediaItem = {
  id: 3,
  path: "/photos/shot.cr2",
  filename: "shot.cr2",
  media_type: "Raw",
  size_bytes: 8192,
  modified_at: "2024-01-01T00:00:00",
};

const heicItem: MediaItem = {
  id: 4,
  path: "/photos/iphone.heic",
  filename: "iphone.heic",
  media_type: "Photo",
  size_bytes: 4096,
  modified_at: "2024-01-01T00:00:00",
};

describe("PhotoCard", () => {
  it("does not show filename overlay", () => {
    render(
      <PhotoCard
        item={photoItem}
        selected={false}
        selectedMediaIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.queryByText("sunset.jpg")).not.toBeInTheDocument();
  });

  it("shows selection indicator when selected", () => {
    const { container } = render(
      <PhotoCard
        item={photoItem}
        selected={true}
        selectedMediaIds={[1]}
        onSelect={vi.fn()}
      />,
    );
    expect(container.querySelector(".bg-blue-500")).toBeInTheDocument();
  });

  it("shows video duration badge for video items", () => {
    render(
      <PhotoCard
        item={videoItem}
        selected={false}
        selectedMediaIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByText("2:05")).toBeInTheDocument();
  });

  it("does not show duration for photo items", () => {
    render(
      <PhotoCard
        item={photoItem}
        selected={false}
        selectedMediaIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.queryByText(/\d+:\d{2}/)).not.toBeInTheDocument();
  });

  it("shows RAW badge for raw items", () => {
    render(
      <PhotoCard
        item={rawItem}
        selected={false}
        selectedMediaIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByText("RAW")).toBeInTheDocument();
  });

  it("does not show RAW badge for photo items", () => {
    render(
      <PhotoCard
        item={photoItem}
        selected={false}
        selectedMediaIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.queryByText("RAW")).not.toBeInTheDocument();
  });

  it("shows HEIC badge for heic items", () => {
    render(
      <PhotoCard
        item={heicItem}
        selected={false}
        selectedMediaIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByText("HEIC")).toBeInTheDocument();
  });

  it("does not show HEIC badge for non-heic photo items", () => {
    render(
      <PhotoCard
        item={photoItem}
        selected={false}
        selectedMediaIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.queryByText("HEIC")).not.toBeInTheDocument();
  });

  it("calls onSelect when clicked", () => {
    const onSelect = vi.fn();
    render(
      <PhotoCard
        item={photoItem}
        selected={false}
        selectedMediaIds={[]}
        onSelect={onSelect}
      />,
    );

    fireEvent.click(screen.getByRole("gridcell"));
    expect(onSelect).toHaveBeenCalledWith(1, expect.any(Object));
  });

  describe("micro-first two-stage loading", () => {
    it("initially renders with micro thumbnail URL", () => {
      render(
        <PhotoCard
          item={photoItem}
          selected={false}
          selectedMediaIds={[]}
          onSelect={vi.fn()}
          thumbnailSize="small"
        />,
      );

      const img = screen.getByRole("img", { hidden: true }) as HTMLImageElement;
      expect(img.src).toContain("/1/micro");
    });

    it("loads full-size thumbnail after micro loads", () => {
      render(
        <PhotoCard
          item={photoItem}
          selected={false}
          selectedMediaIds={[]}
          onSelect={vi.fn()}
          thumbnailSize="small"
        />,
      );

      const microImg = screen.getByRole("img", {
        hidden: true,
      }) as HTMLImageElement;
      expect(microImg.src).toContain("/micro");

      // Simulate micro loaded
      fireEvent.load(microImg);

      // After micro loads, a full-size image should be present
      const images = screen
        .getAllByRole("img", { hidden: true })
        .filter(
          (el) => !(el as HTMLImageElement).src.includes("/micro"),
        ) as HTMLImageElement[];
      expect(images.length).toBeGreaterThan(0);
      expect(images[0].src).toContain("/small");
    });

    it("falls back to small on micro load error", () => {
      render(
        <PhotoCard
          item={photoItem}
          selected={false}
          selectedMediaIds={[]}
          onSelect={vi.fn()}
          thumbnailSize="small"
        />,
      );

      const microImg = screen.getByRole("img", {
        hidden: true,
      }) as HTMLImageElement;
      expect(microImg.src).toContain("/micro");

      // Simulate micro error - should trigger full image load
      fireEvent.error(microImg);

      // After error, full-size image should be present
      const images = screen.getAllByRole("img", {
        hidden: true,
      }) as HTMLImageElement[];
      const fullImg = images.find((img) => img.src.includes("/small"));
      expect(fullImg).toBeDefined();
    });

    it("skips micro for thumbnailSize=large", () => {
      render(
        <PhotoCard
          item={photoItem}
          selected={false}
          selectedMediaIds={[]}
          onSelect={vi.fn()}
          thumbnailSize="large"
        />,
      );

      const img = screen.getByRole("img", { hidden: true }) as HTMLImageElement;
      expect(img.src).toContain("/large");
      expect(img.src).not.toContain("/micro");
    });

    it("shows LIVE badge for LivePhoto media type", () => {
      const liveItem: MediaItem = {
        ...photoItem,
        id: 10,
        media_type: "LivePhoto",
        filename: "IMG_001.heic",
      };
      render(
        <PhotoCard
          item={liveItem}
          selected={false}
          selectedMediaIds={[]}
          onSelect={vi.fn()}
        />,
      );

      expect(screen.getByText("LIVE")).toBeInTheDocument();
    });

    it("does not show LIVE badge for regular photos", () => {
      render(
        <PhotoCard
          item={photoItem}
          selected={false}
          selectedMediaIds={[]}
          onSelect={vi.fn()}
        />,
      );

      expect(screen.queryByText("LIVE")).not.toBeInTheDocument();
    });
  });
});
