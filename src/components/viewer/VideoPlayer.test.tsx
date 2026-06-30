import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { VideoPlayer } from "./VideoPlayer";
import { setLocale } from "@/i18n/index";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();

  Object.defineProperty(HTMLMediaElement.prototype, "play", {
    configurable: true,
    value: vi.fn().mockResolvedValue(undefined),
  });
  Object.defineProperty(HTMLMediaElement.prototype, "pause", {
    configurable: true,
    value: vi.fn(),
  });
});

describe("VideoPlayer", () => {
  it("renders video with play button", () => {
    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[]}
      />,
    );

    expect(document.querySelector("video")).toHaveAttribute(
      "src",
      "file:///videos/test.mp4",
    );
    expect(screen.getByLabelText("播放")).toBeInTheDocument();
    expect(screen.getByText("0:00 / 0:00")).toBeInTheDocument();
  });

  it("toggles play/pause on button click", async () => {
    const user = userEvent.setup();
    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[]}
      />,
    );

    const video = document.querySelector("video")!;
    let paused = true;
    Object.defineProperty(video, "paused", {
      configurable: true,
      get: () => paused,
    });

    await user.click(screen.getByLabelText("播放"));
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalled();
    paused = false;

    fireEvent.play(video);
    expect(screen.getByLabelText("暂停")).toBeInTheDocument();

    await user.click(screen.getByLabelText("暂停"));
    expect(HTMLMediaElement.prototype.pause).toHaveBeenCalled();
  });

  it("toggles play with space key", () => {
    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[]}
      />,
    );

    fireEvent.keyDown(window, { key: " " });
    expect(HTMLMediaElement.prototype.play).toHaveBeenCalled();
  });

  it("seeks with arrow keys", () => {
    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[]}
      />,
    );

    const video = document.querySelector("video")!;
    Object.defineProperty(video, "duration", {
      configurable: true,
      value: 120,
    });
    Object.defineProperty(video, "currentTime", {
      configurable: true,
      value: 30,
      writable: true,
    });
    fireEvent.loadedMetadata(video);
    fireEvent.timeUpdate(video);

    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(video.currentTime).toBe(35);
    fireEvent.timeUpdate(video);

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    expect(video.currentTime).toBe(30);
  });

  it("seeks on progress bar click", () => {
    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[]}
      />,
    );

    const video = document.querySelector("video")!;
    Object.defineProperty(video, "duration", {
      configurable: true,
      value: 100,
    });
    Object.defineProperty(video, "currentTime", {
      configurable: true,
      value: 0,
      writable: true,
    });
    fireEvent.loadedMetadata(video);

    const slider = screen.getAllByRole("slider")[0];
    vi.spyOn(slider, "getBoundingClientRect").mockReturnValue({
      left: 0,
      width: 200,
      top: 0,
      height: 10,
      right: 200,
      bottom: 10,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    });

    fireEvent.click(slider, { clientX: 100 });
    expect(video.currentTime).toBe(50);
  });

  it("renders filmstrip thumbnails", () => {
    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[1, 2, 3]}
      />,
    );

    const thumbs = document.querySelectorAll("img");
    expect(thumbs).toHaveLength(3);
  });

  it("calls onNavigate when filmstrip item clicked", async () => {
    const user = userEvent.setup();
    const onNavigate = vi.fn();

    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[1, 2]}
        onNavigate={onNavigate}
      />,
    );

    const filmstripButtons = screen
      .getAllByRole("button")
      .filter((btn) => btn.className.includes("h-14"));
    await user.click(filmstripButtons[1]);

    expect(onNavigate).toHaveBeenCalledWith(2);
  });

  it("shows mute and fullscreen controls", () => {
    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[]}
      />,
    );

    expect(screen.getByText("静音")).toBeInTheDocument();
    expect(screen.getByText("全屏")).toBeInTheDocument();
  });

  it("toggles mute state", async () => {
    const user = userEvent.setup();
    render(
      <VideoPlayer
        src="file:///videos/test.mp4"
        mediaId={1}
        filmstripIds={[]}
      />,
    );

    const video = document.querySelector("video")!;
    Object.defineProperty(video, "muted", {
      configurable: true,
      value: false,
      writable: true,
    });

    await user.click(screen.getByText("静音"));
    expect(video.muted).toBe(true);
    expect(screen.getByText("取消静音")).toBeInTheDocument();
  });
});
