import { memo, useCallback, useEffect, useMemo, useState } from "react";
import {
  getAiStatus,
  getThumbnailUrl,
  listPersons,
  type AiStatus,
  type Person,
} from "@/lib/tauri";
import { openPersonDetail } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

const CARD_PAGE_SIZE = 20;

function personCoverMediaId(person: Person): number | null {
  return person.sample_media_ids[0] ?? null;
}

interface PersonCardProps {
  person: Person;
  nameLabel: string;
  faceCountLabel: string;
  onOpen: (personId: number) => void;
}

const PersonCard = memo(function PersonCard({
  person,
  nameLabel,
  faceCountLabel,
  onOpen,
}: PersonCardProps) {
  const coverId = personCoverMediaId(person);

  return (
    <button
      type="button"
      onClick={() => onOpen(person.id)}
      className="card-list-item group flex flex-col items-center gap-2 rounded-lg border border-neutral-200/80 dark:border-neutral-800 bg-neutral-100 dark:bg-neutral-900/50 p-4 text-center transition hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800/80"
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
      <div className="min-w-0 w-full">
        <p className="truncate text-sm font-medium text-neutral-100">{nameLabel}</p>
        <p className="mt-0.5 text-xs text-neutral-500">{faceCountLabel}</p>
      </div>
    </button>
  );
});

export function PeopleView() {
  const { t } = useTranslation();
  const [persons, setPersons] = useState<Person[]>([]);
  const [aiStatus, setAiStatus] = useState<AiStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [visibleCount, setVisibleCount] = useState(CARD_PAGE_SIZE);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [people, status] = await Promise.all([listPersons(), getAiStatus()]);
      setPersons(people);
      setAiStatus(status);
      setVisibleCount(CARD_PAGE_SIZE);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const handleOpenPerson = useCallback((personId: number) => {
    openPersonDetail(personId);
  }, []);

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
      <div className="flex items-center justify-between border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-3">
        <h2 className="text-sm font-medium text-neutral-200">{t("people.title")}</h2>
        {aiStatus && (
          <span
            className={`text-xs ${
              aiReady ? "text-green-400" : "text-neutral-500"
            }`}
            title={aiStatus.status_message}
          >
            {t("ai.status")}:{" "}
            {aiReady ? t("ai.available") : t("ai.unavailable")}
          </span>
        )}
      </div>

      {persons.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center text-neutral-500">
          <div className="text-5xl">👤</div>
          <p className="text-lg">{t("people.empty")}</p>
          <p className="max-w-sm text-sm text-neutral-600">
            {!aiStatus?.face_available && t("people.emptyHint")}
          </p>
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
                onOpen={handleOpenPerson}
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
