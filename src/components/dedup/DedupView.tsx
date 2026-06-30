import { memo, useCallback, useEffect, useMemo, useState } from "react";
import {
  getDuplicateGroups,
  getThumbnailUrl,
  runDedupScan,
  resolveDuplicate,
  dismissDuplicateGroup,
  type DuplicateGroup,
  type DuplicateMember,
} from "@/lib/tauri";
import { useTranslation } from "@/i18n/useTranslation";
import { LoadingIndicator } from "@/components/ui/LoadingIndicator";
import { ErrorBanner } from "@/components/ui/ErrorBanner";

const GROUP_PAGE_SIZE = 20;

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
  return `${(bytes / 1073741824).toFixed(1)} GB`;
}

interface DedupMemberCardProps {
  member: DuplicateMember;
  group: DuplicateGroup;
  isKept: boolean;
  onSelect: (groupId: number, mediaId: number) => void;
  similarityLabel: string;
  keptLabel: string;
}

const DedupMemberCard = memo(function DedupMemberCard({
  member,
  group,
  isKept,
  onSelect,
  similarityLabel,
  keptLabel,
}: DedupMemberCardProps) {
  return (
    <button
      type="button"
      onClick={() => onSelect(group.id, member.media_id)}
      aria-label={member.filename}
      aria-pressed={isKept}
      className={`group relative overflow-hidden rounded-lg text-left transition active:scale-[0.98] ${
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
          decoding="async"
          className="h-full w-full object-cover"
        />
      </div>
      <div className="space-y-0.5 p-2">
        <p className="truncate text-xs text-neutral-200">{member.filename}</p>
        <p className="text-xs text-neutral-500">
          {formatFileSize(member.size_bytes)}
        </p>
        {group.match_type === "perceptual" && (
          <p className="text-xs text-neutral-500">{similarityLabel}</p>
        )}
      </div>
      {isKept && (
        <div className="absolute right-1.5 top-1.5 rounded-full bg-green-600 px-2 py-0.5 text-xs font-medium text-white">
          {keptLabel}
        </div>
      )}
    </button>
  );
});

interface DedupGroupCardProps {
  group: DuplicateGroup;
  keepId: number | undefined;
  onKeepSelect: (groupId: number, mediaId: number) => void;
  onDismiss: (groupId: number) => void;
  onResolve: (groupId: number, keepId: number | undefined) => void;
  labels: {
    exact: string;
    perceptual: string;
    count: string;
    dismiss: string;
    delete: string;
    kept: string;
    similarity: (value: number) => string;
  };
}

const DedupGroupCard = memo(function DedupGroupCard({
  group,
  keepId,
  onKeepSelect,
  onDismiss,
  onResolve,
  labels,
}: DedupGroupCardProps) {
  return (
    <div className="dedup-group-card overflow-hidden rounded-xl border border-neutral-200/80 dark:border-neutral-800 bg-neutral-100 dark:bg-neutral-900/50">
      <div className="flex items-center justify-between border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-3">
        <div className="flex items-center gap-3">
          <span
            className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${
              group.match_type === "exact"
                ? "bg-red-900/40 text-red-400"
                : "bg-amber-900/40 text-amber-400"
            }`}
          >
            {group.match_type === "exact" ? labels.exact : labels.perceptual}
          </span>
          <span className="text-sm text-neutral-500 dark:text-neutral-400">
            {labels.count}
          </span>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => onDismiss(group.id)}
            className="rounded-lg px-3 py-1.5 text-xs text-neutral-500 dark:text-neutral-400 transition hover:bg-neutral-100 dark:hover:bg-neutral-800 hover:text-neutral-200"
          >
            {labels.dismiss}
          </button>
          <button
            type="button"
            onClick={() => onResolve(group.id, keepId)}
            className="rounded-lg bg-red-600/80 px-3 py-1.5 text-xs font-medium text-white transition hover:bg-red-600"
          >
            {labels.delete}
          </button>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-[3px] p-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
        {group.members.map((member) => {
          const isKept = member.media_id === keepId;
          return (
            <DedupMemberCard
              key={member.media_id}
              member={member}
              group={group}
              isKept={isKept}
              onSelect={onKeepSelect}
              similarityLabel={labels.similarity(
                Math.round(member.similarity * 100),
              )}
              keptLabel={labels.kept}
            />
          );
        })}
      </div>
    </div>
  );
});

export function DedupView() {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const [hasScanned, setHasScanned] = useState(false);
  const [scanResult, setScanResult] = useState<string | null>(null);
  const [selectedKeep, setSelectedKeep] = useState<Record<number, number>>({});
  const [visibleCount, setVisibleCount] = useState(GROUP_PAGE_SIZE);

  const loadGroups = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const data = await getDuplicateGroups();
      setGroups(data);
      setVisibleCount(GROUP_PAGE_SIZE);
      if (data.length > 0) setHasScanned(true);
    } catch (err) {
      console.error("Failed to load duplicate groups:", err);
      setLoadError(t("errors.generic"));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void loadGroups();
  }, [loadGroups]);

  const handleScan = async () => {
    setScanning(true);
    setScanResult(null);
    setScanError(null);
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
    } catch (err) {
      console.error("Dedup scan failed:", err);
      setScanError(t("errors.generic"));
    } finally {
      setScanning(false);
    }
  };

  const handleResolve = useCallback(
    async (groupId: number, keepId: number | undefined) => {
      if (keepId == null) return;
      if (!window.confirm(t("dedup.confirmDelete"))) return;
      try {
        await resolveDuplicate(groupId, keepId, true);
        setGroups((prev) => prev.filter((g) => g.id !== groupId));
      } catch (err) {
        console.error("Failed to resolve duplicate group:", err);
      }
    },
    [t],
  );

  const handleDismiss = useCallback(async (groupId: number) => {
    try {
      await dismissDuplicateGroup(groupId);
      setGroups((prev) => prev.filter((g) => g.id !== groupId));
    } catch (err) {
      console.error("Failed to dismiss duplicate group:", err);
    }
  }, []);

  const handleKeepSelect = useCallback((groupId: number, mediaId: number) => {
    setSelectedKeep((prev) => ({
      ...prev,
      [groupId]: mediaId,
    }));
  }, []);

  const visibleGroups = useMemo(
    () => groups.slice(0, visibleCount),
    [groups, visibleCount],
  );
  const hasMoreGroups = visibleCount < groups.length;

  const groupLabels = useMemo(
    () => ({
      exact: t("dedup.exact"),
      perceptual: t("dedup.perceptual"),
      dismiss: t("dedup.dismiss"),
      delete: t("dedup.delete"),
      kept: t("dedup.kept"),
      similarity: (value: number) => t("dedup.similarity", { value }),
    }),
    [t],
  );

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-neutral-200/80 dark:border-neutral-800 px-6 py-3">
        <div>
          <h2 className="text-base font-semibold">{t("dedup.title")}</h2>
          {groups.length > 0 && (
            <p className="text-xs text-neutral-500 dark:text-neutral-400">
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
        <div
          role="status"
          className="border-b border-neutral-200/80 dark:border-neutral-800 bg-green-900/20 px-6 py-2 text-sm text-green-400"
        >
          {scanResult}
        </div>
      )}

      {scanError && (
        <ErrorBanner message={scanError} onRetry={() => void handleScan()} />
      )}

      {loadError && !loading && (
        <ErrorBanner message={loadError} onRetry={() => void loadGroups()} />
      )}

      <div className="flex-1 overflow-y-auto px-6 py-4">
        {loading ? (
          <LoadingIndicator className="py-20" label={t("gallery.loading")} />
        ) : groups.length === 0 && !loadError ? (
          <div className="flex flex-1 flex-col items-center justify-center py-20 text-neutral-500">
            <div className="text-5xl">✓</div>
            <p className="mt-4 text-lg">{t("dedup.noDuplicates")}</p>
            <p className="mt-1 text-sm text-neutral-600">
              {t("dedup.noDuplicatesHint")}
            </p>
          </div>
        ) : (
          <div className="space-y-6">
            {visibleGroups.map((group) => {
              const keepId =
                selectedKeep[group.id] ?? group.members[0]?.media_id;
              return (
                <DedupGroupCard
                  key={group.id}
                  group={group}
                  keepId={keepId}
                  onKeepSelect={handleKeepSelect}
                  onDismiss={handleDismiss}
                  onResolve={handleResolve}
                  labels={{
                    ...groupLabels,
                    count: `${group.members.length} ${t("gallery.count", { count: "" }).replace(/\s*$/, "")}`,
                  }}
                />
              );
            })}
            {hasMoreGroups && (
              <div className="flex justify-center pt-2 pb-4">
                <button
                  type="button"
                  onClick={() =>
                    setVisibleCount((prev) => prev + GROUP_PAGE_SIZE)
                  }
                  className="rounded-lg border border-neutral-200/80 dark:border-neutral-700 px-6 py-2 text-sm text-neutral-500 dark:text-neutral-400 transition hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800 hover:text-neutral-200"
                >
                  {t("gallery.loadMore")} ({groups.length - visibleCount})
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
