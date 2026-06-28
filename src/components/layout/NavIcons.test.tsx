import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { NavIconAllPhotos, NavIconVideos, NavIcon } from "./NavIcons";

describe("NavIcons", () => {
  it("merges default size classes with custom className", () => {
    const { container } = render(
      <NavIconAllPhotos className="text-blue-500" data-testid="icon" />,
    );
    const svg = container.querySelector("svg");
    expect(svg).toHaveClass("h-4", "w-4", "shrink-0", "text-blue-500");
  });

  it("passes through additional svg props", () => {
    const { container } = render(<NavIconVideos aria-label="Videos" />);
    expect(container.querySelector("svg")).toHaveAttribute("aria-label", "Videos");
  });

  it("renders named icon via NavIcon wrapper", () => {
    const { container } = render(<NavIcon name="settings" className="opacity-50" />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveClass("h-4", "w-4", "opacity-50");
  });
});
