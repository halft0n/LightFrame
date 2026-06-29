import { memo, useCallback, useEffect, useMemo, useState } from "react";
import {
  clusterFaces,
  detectFacesBatch,
  getAiStatus,
  getThumbnailUrl,
  listPersons,
  mergePersons,
  onFaceDetectionProgress,
  renamePerson,
  type AiStatus,
  type FaceDetectionProgress,
  type Person,
} from "@/lib/tauri";
import { openPersonDetail } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";
import { localizeError } from "@/lib/errors";

const CARD_PAGE_SIZE = 20;

function personCoverMediaId(person: Person): number | null {
  return person.sample_media_ids[0] ?? null;
}

interface PersonCardProps {
  person: Person;
  nameLabel: string;
  faceCountLabel: string;
  selected: boolean;
  onOpen: (personId: number) => void;
  onToggleSelect: (personId: number) => void;
  onRename: (personId: number, name: string) => void;
}

const PersonCard = memo(function PersonCard({
  person,
  nameLabel,
  faceCountLabel,
  selected,
  onOpen,
  onToggleSelect,
  onRename,
}: PersonCardProps) {
  const { t } = useTranslation();
  const coverId = personCoverMediaId(person);
  const [editing, setEditing] = useState(false);
  const [draftName, setDraftName] = useState(nameLabel);

  const commitRename = () => {
    const trimmed = draftName.trim();
    if (trimmed && trimmed !== nameLabel) {
      onRename(person.id, trimmed);
    }
    setEditing(false);
  };

  return (
    <div
      className={`card-list-item group relative flex flex-col items-center gap-2 rounded-lg border p-4 text-center transition ${
        selected
          ? "border-blue-500/60 bg-blue-500/10"
          : "border-neutral-200/80 dark:border-neutral-800 bg-neutral-100 dark:bg-neutral-900/50 hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800/80"
      }`}
    >
      <button
        type="button"
        onClick={() => onToggleSelect(person.id)}
        className="absolute left-2 top-2 flex h-5 w-5 items-center justify-center rounded border border-neutral-500/50 bg-neutral-900/60 text-xs text-white"
        aria-label={t("people.selectPerson")}
      >
        {selected ? "✓" : ""}
      </button>
      <button
        type="button"
        onClick={() => onOpen(person.id)}
        className="flex flex-col items-center gap-2"
      >
        <div className="h-20 w-20 overflow-hidden rounded-full bg-neutral-800 ring-2 ring-neutral-700 transition group-hover:ring-neutral-500">
          {coverId != null ? (
            <img
              src={getThumbnailUrl(coverId, "small")}
              alt=""
              className="card-thumb h-full w-full object-cover"
              loading="lazy"
              decoding="async"
            />
          ) : (
            <div className="flex h-full w-full items-center justify-center text-2xl text-neutral-600">
              👤
            </div>
          )}
        </div>
      </button>
      <div className="min-w-0 w-full">
        {editing ? (
          <input
            type="text"
            value={draftName}
            onChange={(e) => setDraftName(e.target.value)}
            onBlur={commitRename}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitRename();
              if (e.key === "Escape") setEditing(false);
            }}
            className="w-full rounded border border-neutral-600 bg-neutral-900 px-2 py-1 text-center text-sm text-neutral-100"
            autoFocus
          />
        ) : (
          <button
            type="button"
            onClick={() => {
              setDraftName(nameLabel);
              setEditing(true);
            }}
            className="truncate text-sm font-medium text-neutral-100 hover:underline"
            title={t("people.rename")}
          >
            {nameLabel}
          </button>
        )}
        <p className="mt-0.5 text-xs text-neutral-500">{faceCountLabel}</p>
      </div>
    </div>
  );
});

export function PeopleView() {
  const { t } = useTranslation();
  const [persons, setPersons] = useState<Person[]>([]);
  const [aiStatus, setAiStatus] = useState<AiStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [clustering, setClustering] = useState(false);
  const [detecting, setDetecting] = useState(false);
  const [detectionProgress, setDetectionProgress] = useState<FaceDetectionProgress | null>(null);
  const [detectionResult, setDetectionResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [merging, setMerging] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [visibleCount, setVisibleCount] = useState(CARD_PAGE_SIZE);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [people, status] = await Promise.all([listPersons(), getAiStatus()]);
      setPersons(people);
      setAiStatus(status);
      setVisibleCount(CARD_PAGE_SIZE);
    } catch (e) {
      setError(localizeError(e, t));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const handleOpenPerson = useCallback((personId: number) => {
    openPersonDetail(personId);
  }, []);

  const handleToggleSelect = useCallback((personId: number) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(personId)) {
        next.delete(personId);
      } else {
        next.add(personId);
      }
      return next;
    });
  }, []);

  const handleRename = useCallback(async (personId: number, name: string) => {
    try {
      await renamePerson(personId, name);
      setPersons((prev) =>
        prev.map((p) => (p.id === personId ? { ...p, name } : p)),
      );
    } catch (e) {
      setError(localizeError(e, t));
    }
  }, [t]);

  const handleDetectFaces = useCallback(async () => {
    setDetecting(true);
    setDetectionResult(null);
    setDetectionProgress(null);
    setError(null);
    try {
      const result = await detectFacesBatch();
      setDetectionResult(
        t("people.facesFound", { count: result.faces_found }),
      );
      await clusterFaces();
      setSelectedIds(new Set());
      await load();
    } catch (e) {
      setError(localizeError(e, t));
    } finally {
      setDetecting(false);
      setDetectionProgress(null);
    }
  }, [load, t]);

  useEffect(() => {
    if (!detecting) return;

    let unlisten: (() => void) | undefined;
    void onFaceDetectionProgress((progress) => {
      setDetectionProgress(progress);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [detecting]);

  const handleCluster = useCallback(async () => {
    setClustering(true);
    setError(null);
    try {
      await clusterFaces();
      setSelectedIds(new Set());
      await load();
    } catch (e) {
      setError(localizeError(e, t));
    } finally {
      setClustering(false);
    }
  }, [load, t]);

  const handleMerge = useCallback(async () => {
    if (selectedIds.size < 2) return;
    setMerging(true);
    setError(null);
    try {
      await mergePersons([...selectedIds]);
      setSelectedIds(new Set());
      await load();
    } catch (e) {
      setError(localizeError(e, t));
    } finally {
      setMerging(false);
    }
  }, [load, selectedIds, t]);

  const visiblePersons = useMemo(
    () => persons.slice(0, visibleCount),
    [persons, visibleCount],
  );
  const hasMoreCards = visibleCount < persons.length;

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  const aiReady = aiStatus?.face_available ?? false;

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-3">
        <h2 className="text-sm font-medium text-neutral-200">{t("people.title")}</h2>
        <div className="flex flex-wrap items-center gap-2">
          {selectedIds.size >= 2 && (
            <button
              type="button"
              onClick={() => void handleMerge()}
              disabled={merging}
              className="rounded-lg border border-neutral-600 px-3 py-1 text-xs text-neutral-300 transition hover:bg-neutral-800 disabled:opacity-50"
            >
              {merging ? t("people.merging") : t("people.mergeSelected", { count: selectedIds.size })}
            </button>
          )}
          <button
            type="button"
            onClick={() => void handleDetectFaces()}
            disabled={detecting || !aiReady}
            className="rounded-lg border border-neutral-600 px-3 py-1 text-xs text-neutral-300 transition hover:bg-neutral-800 disabled:opacity-50"
          >
            {detecting
              ? detectionProgress && detectionProgress.total > 0
                ? t("people.detectionProgress", {
                    processed: detectionProgress.processed,
                    total: detectionProgress.total,
                  })
                : t("people.detectingFaces")
              : t("people.detectFaces")}
          </button>
          <button
            type="button"
            onClick={() => void handleCluster()}
            disabled={clustering || detecting}
            className="rounded-lg border border-neutral-600 px-3 py-1 text-xs text-neutral-300 transition hover:bg-neutral-800 disabled:opacity-50"
          >
            {clustering ? t("people.clustering") : t("people.clusterFaces")}
          </button>
          {detectionResult && !detecting && (
            <span className="text-xs text-green-400">{detectionResult}</span>
          )}
          {aiStatus && (
            <span
              className={`text-xs ${aiReady ? "text-green-400" : "text-neutral-500"}`}
              title={aiStatus.status_message}
            >
              {t("ai.status")}: {aiReady ? t("ai.available") : t("ai.unavailable")}
            </span>
          )}
        </div>
      </div>

      {error && (
        <div className="border-b border-red-900/50 bg-red-950/30 px-4 py-2">
          <p className="text-sm text-red-400">{error}</p>
        </div>
      )}

      {persons.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center text-neutral-500">
          <div className="text-5xl">👤</div>
          <p className="text-lg">{t("people.empty")}</p>
          <p className="max-w-sm text-sm text-neutral-600">
            {!aiStatus?.face_available && t("people.emptyHint")}
          </p>
          <button
            type="button"
            onClick={() => void handleDetectFaces()}
            disabled={detecting || !aiReady}
            className="rounded-lg border border-neutral-600 px-4 py-2 text-sm text-neutral-300 transition hover:bg-neutral-800 disabled:opacity-50"
          >
            {detecting ? t("people.detectingFaces") : t("people.detectFaces")}
          </button>
          <button
            type="button"
            onClick={() => void handleCluster()}
            disabled={clustering || detecting}
            className="rounded-lg border border-neutral-600 px-4 py-2 text-sm text-neutral-300 transition hover:bg-neutral-800 disabled:opacity-50"
          >
            {clustering ? t("people.clustering") : t("people.clusterFaces")}
          </button>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto px-1 py-1">
          <div className="grid gap-[3px] sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
            {visiblePersons.map((person) => (
              <PersonCard
                key={person.id}
                person={person}
                nameLabel={person.name ?? t("people.unnamed")}
                faceCountLabel={t("people.faceCount", { count: person.face_count })}
                selected={selectedIds.has(person.id)}
                onOpen={handleOpenPerson}
                onToggleSelect={handleToggleSelect}
                onRename={handleRename}
              />
            ))}
          </div>
          {hasMoreCards && (
            <div className="flex justify-center py-6">
              <button
                type="button"
                onClick={() => setVisibleCount((prev) => prev + CARD_PAGE_SIZE)}
                className="rounded-lg border border-neutral-200/80 dark:border-neutral-700 px-6 py-2 text-sm text-neutral-500 dark:text-neutral-400 transition hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800 hover:text-neutral-200"
              >
                {t("gallery.loadMore")} ({persons.length - visibleCount})
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
