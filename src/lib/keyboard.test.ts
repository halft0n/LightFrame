import { describe, it, expect } from "vitest";
import { isTypingTarget } from "./keyboard";

describe("isTypingTarget", () => {
  it("returns true for input elements", () => {
    const input = document.createElement("input");
    expect(isTypingTarget(input)).toBe(true);
  });

  it("returns true for textarea elements", () => {
    const textarea = document.createElement("textarea");
    expect(isTypingTarget(textarea)).toBe(true);
  });

  it("returns true for select elements", () => {
    const select = document.createElement("select");
    expect(isTypingTarget(select)).toBe(true);
  });

  it("returns true for contenteditable elements", () => {
    const div = document.createElement("div");
    div.contentEditable = "true";
    document.body.appendChild(div);
    expect(isTypingTarget(div)).toBe(true);
    document.body.removeChild(div);
  });

  it("returns false for div, button, and other non-editable elements", () => {
    expect(isTypingTarget(document.createElement("div"))).toBe(false);
    expect(isTypingTarget(document.createElement("button"))).toBe(false);
    expect(isTypingTarget(document.createElement("span"))).toBe(false);
    expect(isTypingTarget(document.createElement("a"))).toBe(false);
  });

  it("returns false for null and non-HTMLElement targets", () => {
    expect(isTypingTarget(null)).toBe(false);
    expect(isTypingTarget(document)).toBe(false);
    expect(isTypingTarget(window)).toBe(false);
  });
});
