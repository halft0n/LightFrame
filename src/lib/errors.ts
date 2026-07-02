type TranslateFn = (
  key: string,
  params?: Record<string, string | number>,
) => string;

export function localizeError(error: unknown, t: TranslateFn): string {
  const msg = error instanceof Error ? error.message : String(error);
  if (msg.includes("not found")) return t("errors.notFound");
  if (msg.includes("database")) return t("errors.database");
  if (msg.includes("permission") || msg.includes("forbidden"))
    return t("errors.forbidden");
  if (msg.includes("too large")) return t("errors.tooLarge");
  if (msg.includes("batch size")) return t("errors.batchTooLarge");
  if (msg.includes("download failed") || msg.includes("network error"))
    return localizeDownloadError(msg, t);
  if (msg.includes("already downloading"))
    return t("errors.alreadyDownloading");
  return t("errors.generic");
}

function localizeDownloadError(msg: string, t: TranslateFn): string {
  if (msg.includes("Dns")) return t("errors.downloadDns");
  if (msg.includes("ConnectionFailed") || msg.includes("ConnectionReset"))
    return t("errors.downloadConnection");
  if (msg.includes("Timeout") || msg.includes("timeout"))
    return t("errors.downloadTimeout");
  if (msg.includes("HTTP 4")) return t("errors.downloadClientError");
  if (msg.includes("HTTP 5")) return t("errors.downloadServerError");
  return t("errors.downloadGeneric");
}
