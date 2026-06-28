import { useCallback, useEffect, useState } from "react";
import {
  getDuplicateGroups,
  getThumbnailUrl,
  runDedupScan,
  resolveDuplicate,
  dismissDuplicateGroup,
  type DuplicateGroup,
} from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
  return `${(bytes / 1073741824).toFixed(1)} GB`;
}

export function DedupView() {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
  const [scanning, setScanning] = useState(false);
  const [hasScanned, setHasScanned] = useState(false);
  const [scanResult, setScanResult] = useState<string | null>(null);
  const [selectedKeep, setSelectedKeep] = useState<Record<number, number>>({});

  const loadGroups = useCallback(async () => {
    try {
      const data = await getDuplicateGroups();
      setGroups(data);
      if (data.length > 0) setHasScanned(true);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    void loadGroups();
  }, [loadGroups]);

  const handleScan = async () => {
    setScanning(true);
    setScanResult(null);
    try {
      const result = await runDedupScan();
      setScanResult(
        t("dedup.scanComplete", {
          exact: result.exact_groups,
          perceptual: result.perceptual_groups,
        }),
      );
      setHasScanned(true);
      await loadGroups();
    } catch {
      // ignore
    } finally {
      setScanning(false);
    }
  };

  const handleResolve = async (groupId: number, keepId: number | undefined) => {
    if (keepId == null) return;
    try {
      await resolveDuplicate(groupId, keepId, true);
      setGroups((prev) => prev.filter((g) => g.id !== groupId));
    } catch {
      // ignore
    }
  };

  const handleDismiss = async (groupId: number) => {
    try {
      await dismissDuplicateGroup(groupId);
      setGroups((prev) => prev.filter((g) => g.id !== groupId));
    } catch {
      // ignore
    }
  };

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-neutral-800 px-6 py-3">
        <div>
          <h2 className="text-base font-semibold">{t("dedup.title")}</h2>
          {groups.length > 0 && (
            <p className="text-xs text-neutral-400">
              {t("dedup.groupCount", { count: groups.length })}
            </p>
          )}
        </div>
        <button
          type="button"
          disabled={scanning}
          onClick={handleScan}
          className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-500 disabled:opacity-50"
        >
          {scanning
            ? t("dedup.scanning")
            : hasScanned
              ? t("dedup.rescan")
              : t("dedup.scan")}
        </button>
      </div>

      {scanResult && (
        <div className="border-b border-neutral-800 bg-green-900/20 px-6 py-2 text-sm text-green-400">
          {scanResult}
        </div>
      )}

      <div className="flex-1 overflow-y-auto px-6 py-4">
        {groups.length === 0 ? (
          <div className="flex flex-1 flex-col items-center justify-center py-20 text-neutral-500">
            <div className="text-5xl">✓</div>
            <p className="mt-4 text-lg">{t("dedup.noDuplicates")}</p>
            <p className="mt-1 text-sm text-neutral-600">
              {t("dedup.noDuplicatesHint")}
            </p>
          </div>
        ) : (
          <div className="space-y-6">
            {groups.map((group) => {
              const keepId = selectedKeep[group.id] ?? group.members[0]?.media_id;
              return (
                <div
                  key={group.id}
                  className="overflow-hidden rounded-xl border border-neutral-800 bg-neutral-900/50"
                >
                  <div className="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
                    <div className="flex items-center gap-3">
                      <span
                        className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${
                          group.match_type === "exact"
                            ? "bg-red-900/40 text-red-400"
                            : "bg-amber-900/40 text-amber-400"
                        }`}
                      >
                        {group.match_type === "exact"
                          ? t("dedup.exact")
                          : t("dedup.perceptual")}
                      </span>
                      <span className="text-sm text-neutral-400">
                        {group.members.length} {t("gallery.count", { count: "" }).replace(/\s*$/, "")}
                      </span>
                    </div>
                    <div className="flex gap-2">
                      <button
                        type="button"
                        onClick={() => handleDismiss(group.id)}
                        className="rounded-lg px-3 py-1.5 text-xs text-neutral-400 transition hover:bg-neutral-800 hover:text-neutral-200"
                      >
                        {t("dedup.dismiss")}
                      </button>
                      <button
                        type="button"
                        onClick={() => handleResolve(group.id, keepId)}
                        className="rounded-lg bg-red-600/80 px-3 py-1.5 text-xs font-medium text-white transition hover:bg-red-600"
                      >
                        {t("dedup.delete")}
                      </button>
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-3 p-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
                    {group.members.map((member) => {
                      const isKept = member.media_id === keepId;
                      return (
                        <button
                          key={member.media_id}
                          type="button"
                          onClick={() =>
                            setSelectedKeep((prev) => ({
                              ...prev,
                              [group.id]: member.media_id,
                            }))
                          }
                          className={`group relative overflow-hidden rounded-lg text-left transition ${
                            isKept
                              ? "ring-2 ring-green-500 ring-offset-2 ring-offset-neutral-950"
                              : "opacity-70 hover:opacity-100"
                          }`}
                        >
                          <div className="aspect-square overflow-hidden bg-neutral-800">
                            <img
                              src={getThumbnailUrl(member.media_id, "small")}
                              alt={member.filename}
                              loading="lazy"
                              className="h-full w-full object-cover"
                            />
                          </div>
                          <div className="space-y-0.5 p-2">
                            <p className="truncate text-xs text-neutral-200">
                              {member.filename}
                            </p>
                            <p className="text-xs text-neutral-500">
                              {formatFileSize(member.size_bytes)}
                            </p>
                            {group.match_type === "perceptual" && (
                              <p className="text-xs text-neutral-500">
                                {t("dedup.similarity", {
                                  value: Math.round(member.similarity * 100),
                                })}
                              </p>
                            )}
                          </div>
                          {isKept && (
                            <div className="absolute right-1.5 top-1.5 rounded-full bg-green-600 px-2 py-0.5 text-xs font-medium text-white">
                              {t("dedup.kept")}
                            </div>
                          )}
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
