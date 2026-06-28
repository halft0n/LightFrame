import { describe, it, expect } from "vitest";
import {
  DRAG_MEDIA_MIME,
  setDragMediaIds,
  parseDragMediaIds,
  dragMediaIdsForItem,
} from "./dragMedia";

describe("dragMedia", () => {
  describe("DRAG_MEDIA_MIME", () => {
    it("uses the expected custom MIME type", () => {
      expect(DRAG_MEDIA_MIME).toBe("application/x-catchlight-media-ids");
    });
  });

  describe("setDragMediaIds / parseDragMediaIds", () => {
    it("round-trips media IDs through DataTransfer", () => {
      const stored: Record<string, string> = {};
      const dataTransfer = {
        stored,
        effectAllowed: "",
        setData(type: string, value: string) {
          stored[type] = value;
        },
        getData(type: string) {
          return stored[type] ?? "";
        },
      } as unknown as DataTransfer;

      setDragMediaIds(dataTransfer, [1, 2, 3]);
      expect(dataTransfer.effectAllowed).toBe("copy");
      expect(parseDragMediaIds(dataTransfer)).toEqual([1, 2, 3]);
    });

    it("returns empty array when MIME data is missing", () => {
      const dataTransfer = {
        getData: () => "",
      } as unknown as DataTransfer;
      expect(parseDragMediaIds(dataTransfer)).toEqual([]);
    });

    it("returns empty array for invalid JSON", () => {
      const dataTransfer = {
        getData: () => "not-json",
      } as unknown as DataTransfer;
      expect(parseDragMediaIds(dataTransfer)).toEqual([]);
    });

    it("filters non-number entries from parsed array", () => {
      const dataTransfer = {
        getData: () => JSON.stringify([1, "two", 3, null]),
      } as unknown as DataTransfer;
      expect(parseDragMediaIds(dataTransfer)).toEqual([1, 3]);
    });

    it("returns empty array when parsed value is not an array", () => {
      const dataTransfer = {
        getData: () => JSON.stringify({ ids: [1, 2] }),
      } as unknown as DataTransfer;
      expect(parseDragMediaIds(dataTransfer)).toEqual([]);
    });
  });

  describe("dragMediaIdsForItem", () => {
    it("returns all selected IDs when item is in selection", () => {
      expect(dragMediaIdsForItem(2, [1, 2, 3])).toEqual([1, 2, 3]);
    });

    it("returns single item ID when not in selection", () => {
      expect(dragMediaIdsForItem(5, [1, 2, 3])).toEqual([5]);
    });

    it("returns single item when selection is empty", () => {
      expect(dragMediaIdsForItem(7, [])).toEqual([7]);
    });
  });
});
