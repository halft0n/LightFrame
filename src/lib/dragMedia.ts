export const DRAG_MEDIA_MIME = "application/x-lightframe-media-ids";
export const DRAG_PERSON_MIME = "application/x-lightframe-person-id";

export function setDragPersonId(
  dataTransfer: DataTransfer,
  personId: number,
): void {
  dataTransfer.setData(DRAG_PERSON_MIME, String(personId));
  dataTransfer.effectAllowed = "move";
}

export function parseDragPersonId(dataTransfer: DataTransfer): number | null {
  const raw = dataTransfer.getData(DRAG_PERSON_MIME);
  if (!raw) return null;
  const id = Number(raw);
  return Number.isFinite(id) ? id : null;
}

export function setDragMediaIds(
  dataTransfer: DataTransfer,
  mediaIds: number[],
): void {
  dataTransfer.setData(DRAG_MEDIA_MIME, JSON.stringify(mediaIds));
  dataTransfer.effectAllowed = "copy";
}

export function parseDragMediaIds(dataTransfer: DataTransfer): number[] {
  const raw = dataTransfer.getData(DRAG_MEDIA_MIME);
  if (!raw) return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((id): id is number => typeof id === "number");
  } catch {
    return [];
  }
}

export function dragMediaIdsForItem(
  itemId: number,
  selectedIds: number[],
): number[] {
  if (selectedIds.includes(itemId) && selectedIds.length > 0) {
    return selectedIds;
  }
  return [itemId];
}
