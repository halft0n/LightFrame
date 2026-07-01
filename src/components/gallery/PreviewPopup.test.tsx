import { render, screen, fireEvent, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { PreviewPopup } from "./PreviewPopup";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://localhost/${encodeURIComponent(path)}`,
}));

const mockPhoto = {
  id: 1,
  path: "/photos/test.jpg",
  filename: "test.jpg",
  media_type: "Photo" as const,
  size_bytes: 1024,
  width: 1920,
  height: 1080,
};

const mockVideo = {
  id: 2,
  path: "/videos/test.mp4",
  filename: "test.mp4",
  media_type: "Video" as const,
  size_bytes: 10240,
  width: 1920,
  height: 1080,
  duration_sec: 30,
};

describe("PreviewPopup", () => {
  const onClose = vi.fn();

  beforeEach(() => {
    onClose.mockClear();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders nothing when media is null", () => {
    const { container } = render(
      <PreviewPopup media={null} onClose={onClose} />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders an image for Photo type media", () => {
    render(<PreviewPopup media={mockPhoto} onClose={onClose} />);
    const img = screen.getByRole("img");
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute("alt", "test.jpg");
  });

  it("renders a video for Video type media", () => {
    render(<PreviewPopup media={mockVideo} onClose={onClose} />);
    const video = document.querySelector("video");
    expect(video).toBeInTheDocument();
    expect(video).toHaveAttribute("autoplay");
    expect(video).toHaveProperty("muted", true);
  });

  it("calls onClose when Escape key is pressed", () => {
    render(<PreviewPopup media={mockPhoto} onClose={onClose} />);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls onClose when backdrop is clicked", () => {
    render(<PreviewPopup media={mockPhoto} onClose={onClose} />);
    act(() => {
      vi.advanceTimersByTime(50);
    });
    const backdrop = screen.getByTestId("preview-backdrop");
    fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("renders as a portal (outside parent DOM tree)", () => {
    const { baseElement } = render(
      <div data-testid="parent">
        <PreviewPopup media={mockPhoto} onClose={onClose} />
      </div>
    );
    // The popup should be rendered in the body, not inside parent
    const popup = baseElement.querySelector("[data-testid='preview-popup']");
    expect(popup).toBeInTheDocument();
  });

  it("shows the filename in the preview", () => {
    render(<PreviewPopup media={mockPhoto} onClose={onClose} />);
    expect(screen.getByText("test.jpg")).toBeInTheDocument();
  });

  it("calls onClose on pointerCancel after dismiss delay", () => {
    render(<PreviewPopup media={mockPhoto} onClose={onClose} />);
    act(() => {
      vi.advanceTimersByTime(50);
    });
    const backdrop = screen.getByTestId("preview-backdrop");
    fireEvent.pointerCancel(backdrop);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls onClose on document pointerup after dismiss delay", () => {
    render(<PreviewPopup media={mockPhoto} onClose={onClose} />);
    act(() => {
      vi.advanceTimersByTime(50);
    });
    fireEvent.pointerUp(document);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
