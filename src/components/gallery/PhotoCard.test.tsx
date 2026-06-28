import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PhotoCard } from "./PhotoCard";
import type { MediaItem } from "@/lib/tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
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

describe("PhotoCard", () => {
  it("renders filename on hover overlay", () => {
    render(<PhotoCard item={photoItem} selected={false} onSelect={vi.fn()} />);
    expect(screen.getByText("sunset.jpg")).toBeInTheDocument();
  });

  it("shows video duration badge for video items", () => {
    render(<PhotoCard item={videoItem} selected={false} onSelect={vi.fn()} />);
    expect(screen.getByText("2:05")).toBeInTheDocument();
  });

  it("does not show duration for photo items", () => {
    render(<PhotoCard item={photoItem} selected={false} onSelect={vi.fn()} />);
    expect(screen.queryByText(/\d+:\d{2}/)).not.toBeInTheDocument();
  });

  it("calls onSelect when clicked", () => {
    const onSelect = vi.fn();
    render(<PhotoCard item={photoItem} selected={false} onSelect={onSelect} />);

    fireEvent.click(screen.getByRole("button"));
    expect(onSelect).toHaveBeenCalledWith(1, expect.any(Object));
  });
});
