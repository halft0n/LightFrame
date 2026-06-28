import { useCallback, useEffect, useState } from "react";
import {
  getAiStatus,
  getThumbnailUrl,
  listPersons,
  type AiStatus,
  type Person,
} from "@/lib/tauri";
import { openPersonDetail } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

function personCoverMediaId(person: Person): number | null {
  return person.sample_media_ids[0] ?? null;
}

export function PeopleView() {
  const { t } = useTranslation();
  const [persons, setPersons] = useState<Person[]>([]);
  const [aiStatus, setAiStatus] = useState<AiStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [people, status] = await Promise.all([listPersons(), getAiStatus()]);
      setPersons(people);
      setAiStatus(status);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

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
      <div className="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
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
        <div className="flex-1 overflow-y-auto px-4 py-3">
          <div className="grid gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
            {persons.map((person) => {
              const coverId = personCoverMediaId(person);
              return (
                <button
                  key={person.id}
                  type="button"
                  onClick={() => openPersonDetail(person.id)}
                  className="group flex flex-col items-center gap-2 rounded-lg border border-neutral-800 bg-neutral-900/50 p-4 text-center transition hover:border-neutral-600 hover:bg-neutral-800/80"
                >
                  <div className="h-20 w-20 overflow-hidden rounded-full bg-neutral-800 ring-2 ring-neutral-700 transition group-hover:ring-neutral-500">
                    {coverId != null ? (
                      <img
                        src={getThumbnailUrl(coverId, "small")}
                        alt=""
                        className="h-full w-full object-cover"
                        loading="lazy"
                      />
                    ) : (
                      <div className="flex h-full w-full items-center justify-center text-2xl text-neutral-600">
                        👤
                      </div>
                    )}
                  </div>
                  <div className="min-w-0 w-full">
                    <p className="truncate text-sm font-medium text-neutral-100">
                      {person.name ?? t("people.unnamed")}
                    </p>
                    <p className="mt-0.5 text-xs text-neutral-500">
                      {t("people.faceCount", { count: person.face_count })}
                    </p>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
