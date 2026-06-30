import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { MainContent } from "./MainContent";
import { setLocale } from "@/i18n/index";

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
});

describe("MainContent", () => {
  it("renders welcome message in Chinese", () => {
    render(<MainContent />);
    expect(screen.getByText("欢迎使用 影迹")).toBeInTheDocument();
  });

  it("renders add folder hint in Chinese", () => {
    render(<MainContent />);
    expect(screen.getByText("添加文件夹开始浏览照片")).toBeInTheDocument();
  });

  it("renders in English when locale is en", () => {
    setLocale("en");
    render(<MainContent />);
    expect(screen.getByText("Welcome to LightFrame")).toBeInTheDocument();
    expect(
      screen.getByText("Add a folder to start browsing photos"),
    ).toBeInTheDocument();
  });

  it("renders welcome illustration", () => {
    render(<MainContent />);
    expect(document.querySelector(".empty-state-icon")).toBeInTheDocument();
  });
});
