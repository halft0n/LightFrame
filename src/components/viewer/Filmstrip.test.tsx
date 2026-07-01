import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Filmstrip } from "./Filmstrip";
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

const makeItem = (id: number): MediaItem => ({
  id,
  path: `/photos/img_${id}.jpg`,
  filename: `img_${id}.jpg`,
  media_type: "Photo",
  size_bytes: 1024,
  modified_at: "2024-01-01T00:00:00",
});

describe("Filmstrip", () => {
  const items = Array.from({ length: 20 }, (_, i) => makeItem(i + 1));
  let onNavigate: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onNavigate = vi.fn();
  });

  it("renders nothing when items list is empty", () => {
    const { container } = render(
      <Filmstrip items={[]} currentId={1} onNavigate={onNavigate} />,
    );
    expect(container.querySelector("[role='tablist']")).not.toBeInTheDocument();
  });

  it("renders the correct number of thumbnail buttons", () => {
    render(
      <Filmstrip items={items} currentId={5} onNavigate={onNavigate} />,
    );
    const tabs = screen.getAllByRole("tab");
    expect(tabs.length).toBe(items.length);
  });

  it("highlights the current media item", () => {
    render(
      <Filmstrip items={items} currentId={5} onNavigate={onNavigate} />,
    );
    const selected = screen.getByRole("tab", { selected: true });
    expect(selected).toHaveAttribute("data-id", "5");
  });

  it("calls onNavigate when a thumbnail is clicked", () => {
    render(
      <Filmstrip items={items} currentId={5} onNavigate={onNavigate} />,
    );
    const tabs = screen.getAllByRole("tab");
    fireEvent.click(tabs[2]); // item id=3
    expect(onNavigate).toHaveBeenCalledWith(3);
  });

  it("uses micro thumbnail URL for images", () => {
    const { container } = render(
      <Filmstrip items={items} currentId={1} onNavigate={onNavigate} />,
    );
    const imgs = container.querySelectorAll("img");
    expect(imgs.length).toBeGreaterThan(0);
    const firstImg = imgs[0] as HTMLImageElement;
    expect(firstImg.src).toContain("/micro");
  });

  it("has role=tablist with aria-label", () => {
    render(
      <Filmstrip items={items} currentId={1} onNavigate={onNavigate} />,
    );
    const tablist = screen.getByRole("tablist");
    expect(tablist).toHaveAttribute("aria-label");
  });

  it("is not rendered when visible prop is false", () => {
    const { container } = render(
      <Filmstrip
        items={items}
        currentId={1}
        onNavigate={onNavigate}
        visible={false}
      />,
    );
    expect(container.querySelector("[role='tablist']")).not.toBeInTheDocument();
  });

  it("renders when visible prop is true (default)", () => {
    render(
      <Filmstrip items={items} currentId={1} onNavigate={onNavigate} />,
    );
    expect(screen.getByRole("tablist")).toBeInTheDocument();
  });

  it("scrolls current item into view on currentId change", () => {
    const scrollIntoViewMock = vi.fn();
    Element.prototype.scrollIntoView = scrollIntoViewMock;

    const { rerender } = render(
      <Filmstrip items={items} currentId={1} onNavigate={onNavigate} />,
    );

    rerender(
      <Filmstrip items={items} currentId={10} onNavigate={onNavigate} />,
    );

    expect(scrollIntoViewMock).toHaveBeenCalled();
  });

  it("does not call onNavigate for already-active item", () => {
    render(
      <Filmstrip items={items} currentId={5} onNavigate={onNavigate} />,
    );
    const activeTab = screen.getByRole("tab", { selected: true });
    fireEvent.click(activeTab);
    expect(onNavigate).not.toHaveBeenCalled();
  });
});
