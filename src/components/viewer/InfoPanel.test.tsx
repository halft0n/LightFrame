import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { InfoPanel } from "./InfoPanel";
import { setLocale } from "@/i18n/index";
import type { MediaItem } from "@/lib/tauri";

const baseMedia: MediaItem = {
  id: 1,
  path: "/photos/test.jpg",
  filename: "test.jpg",
  media_type: "Photo",
  size_bytes: 1048576,
  width: 1920,
  height: 1080,
  created_at: "2024-06-15T10:00:00",
  modified_at: "2024-06-15T10:00:00",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
});

describe("InfoPanel", () => {
  it("renders media information correctly", () => {
    render(<InfoPanel media={baseMedia} />);

    expect(
      screen.getByRole("complementary", { name: "信息" }),
    ).toBeInTheDocument();
    expect(screen.getByText("test.jpg")).toBeInTheDocument();
    expect(screen.getByText("/photos/test.jpg")).toBeInTheDocument();
    expect(screen.getByText("1920 × 1080")).toBeInTheDocument();
    expect(screen.getByText("1.0 MB")).toBeInTheDocument();
    expect(screen.getByText("Photo")).toBeInTheDocument();
  });

  it("shows camera info when make and model are available", () => {
    render(
      <InfoPanel
        media={{
          ...baseMedia,
          camera_make: "Canon",
          camera_model: "EOS R5",
        }}
      />,
    );

    expect(screen.getByText("相机")).toBeInTheDocument();
    expect(screen.getByText("Canon EOS R5")).toBeInTheDocument();
  });

  it("hides camera section when camera info is unavailable", () => {
    render(<InfoPanel media={baseMedia} />);
    expect(screen.queryByText("相机")).not.toBeInTheDocument();
  });

  it("shows only make when model is missing", () => {
    render(
      <InfoPanel
        media={{
          ...baseMedia,
          camera_make: "Sony",
        }}
      />,
    );

    expect(screen.getByText("Sony")).toBeInTheDocument();
  });

  it("displays hash values in truncated and hex formats", () => {
    const longHash = "a".repeat(64);
    render(
      <InfoPanel
        media={{
          ...baseMedia,
          blake3_hash: longHash,
          dhash: 0xdeadbeefcafebabe,
          phash: 0x0123456789abcdef,
        }}
      />,
    );

    expect(screen.getByText(`${"a".repeat(16)}…`)).toBeInTheDocument();
    expect(screen.getByText("DHash")).toBeInTheDocument();
    expect(screen.getByText("PHash")).toBeInTheDocument();
    expect(screen.getByText("0xdeadbeefcafeb800")).toBeInTheDocument();
    expect(screen.getByText("0x0123456789abcdf0")).toBeInTheDocument();
  });

  it("handles missing optional fields gracefully", () => {
    render(
      <InfoPanel
        media={{
          id: 2,
          path: "/photos/minimal.jpg",
          filename: "minimal.jpg",
          media_type: "Photo",
          size_bytes: 512,
          modified_at: "2024-01-01T00:00:00",
        }}
      />,
    );

    expect(screen.queryByText(/×/)).not.toBeInTheDocument();
    expect(screen.queryByText("GPS 坐标")).not.toBeInTheDocument();
    expect(screen.queryByText("BLAKE3 哈希")).not.toBeInTheDocument();
    expect(screen.getByText("512 B")).toBeInTheDocument();
  });

  it("shows GPS coordinates when available", () => {
    render(
      <InfoPanel
        media={{
          ...baseMedia,
          latitude: 37.774929,
          longitude: -122.419418,
        }}
      />,
    );

    expect(screen.getByText("GPS 坐标")).toBeInTheDocument();
    expect(screen.getByText("37.774929, -122.419418")).toBeInTheDocument();
  });

  it("shows em dash when no date is available", () => {
    render(
      <InfoPanel
        media={{
          ...baseMedia,
          created_at: null,
          modified_at: "",
        }}
      />,
    );

    const dateRow = screen.getByText("拍摄日期").closest("div");
    expect(dateRow?.querySelector("dd")).toHaveTextContent("—");
  });
});
