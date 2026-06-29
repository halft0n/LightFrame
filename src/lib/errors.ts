type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

export function localizeError(error: unknown, t: TranslateFn): string {
  const msg = error instanceof Error ? error.message : String(error);
  if (msg.includes("not found")) return t("errors.notFound");
  if (msg.includes("database")) return t("errors.database");
  if (msg.includes("permission") || msg.includes("forbidden")) return t("errors.forbidden");
  if (msg.includes("too large")) return t("errors.tooLarge");
  if (msg.includes("batch size")) return t("errors.batchTooLarge");
  return t("errors.generic");
}
