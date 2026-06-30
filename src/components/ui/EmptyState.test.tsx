import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { EmptyState } from "./EmptyState";

describe("EmptyState", () => {
  it("renders with title and description", () => {
    render(
      <EmptyState
        title="No photos yet"
        description="Add a folder to get started"
      />,
    );

    expect(
      screen.getByRole("heading", { name: "No photos yet" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Add a folder to get started")).toBeInTheDocument();
  });

  it("renders with optional icon for variant", () => {
    const { container } = render(
      <EmptyState variant="welcome" title="Welcome" />,
    );

    expect(container.querySelector(".empty-state-icon")).toBeInTheDocument();
  });

  it("renders with action button", () => {
    render(
      <EmptyState
        title="Empty folder"
        action={<button type="button">Add folder</button>}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Add folder" }),
    ).toBeInTheDocument();
  });
});
