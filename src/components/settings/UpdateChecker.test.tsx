import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { UpdateChecker } from "./UpdateChecker";
import { setLocale } from "@/i18n/index";

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();
});

describe("UpdateChecker", () => {
  it("renders title and check button", () => {
    render(<UpdateChecker />);
    expect(screen.getByText("检查更新")).toBeInTheDocument();
  });

  it("opens release page when button is clicked", async () => {
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    expect(openSpy).toHaveBeenCalledWith(
      "https://github.com/halft0n/LightFrame/releases",
      "_blank",
    );
    openSpy.mockRestore();
  });
});
