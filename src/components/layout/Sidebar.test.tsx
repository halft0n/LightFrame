import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { Sidebar } from "./Sidebar";
import { setLocale } from "@/i18n/index";

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
});

describe("Sidebar", () => {
  it("renders the library section header", () => {
    render(<Sidebar />);
    expect(screen.getAllByText("基础图库").length).toBeGreaterThanOrEqual(1);
  });

  it("renders library section in Chinese", () => {
    render(<Sidebar />);
    expect(screen.getByText("所有照片")).toBeInTheDocument();
    expect(screen.getByText("视频")).toBeInTheDocument();
    expect(screen.getByText("收藏")).toBeInTheDocument();
    expect(screen.getByText("地点")).toBeInTheDocument();
    expect(screen.getByText("人物")).toBeInTheDocument();
  });

  it("renders tools section in Chinese", () => {
    render(<Sidebar />);
    expect(screen.getByText("重复照片")).toBeInTheDocument();
    expect(screen.getByText("截图")).toBeInTheDocument();
  });

  it("renders settings button", () => {
    render(<Sidebar />);
    expect(screen.getByText("设置")).toBeInTheDocument();
  });

  it("renders in English when locale is en", () => {
    setLocale("en");
    render(<Sidebar />);
    expect(screen.getByText("All Photos")).toBeInTheDocument();
    expect(screen.getByText("Videos")).toBeInTheDocument();
    expect(screen.getByText("Duplicates")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("renders all navigation items as buttons", () => {
    render(<Sidebar />);
    const buttons = screen.getAllByRole("button");
    expect(buttons.length).toBeGreaterThanOrEqual(7);
  });
});
