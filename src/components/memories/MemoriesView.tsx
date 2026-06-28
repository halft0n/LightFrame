import { useCallback, useEffect, useState } from "react";
import {
  generateMemories,
  getThumbnailUrl,
  listMemories,
  type Memory,
} from "@/lib/tauri";
import { openMemoryDetail } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

export function MemoriesView() {
  const { t } = useTranslation();
  const [memories, setMemories] = useState<Memory[]>([]);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);

  const loadMemories = useCallback(async () => {
    setLoading(true);
    try {
      setMemories(await listMemories());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadMemories();
  }, [loadMemories]);

  const handleGenerate = async () => {
    setGenerating(true);
    try {
      setMemories(await generateMemories());
    } finally {
      setGenerating(false);
    }
  };

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <p>{t("gallery.loading")}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-neutral-200/80 dark:border-neutral-800 px-4 py-3">
        <h2 className="text-sm font-medium text-neutral-200">{t("memories.title")}</h2>
        <button
          type="button"
          disabled={generating}
          onClick={() => void handleGenerate()}
          className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white transition hover:bg-blue-500 disabled:opacity-50"
        >
          {t("memories.generate")}
        </button>
      </div>

      {memories.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-neutral-500">
          <div className="text-5xl">🕰</div>
          <p className="text-lg">{t("memories.empty")}</p>
          <p className="text-sm text-neutral-600">{t("memories.emptyHint")}</p>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto px-1 py-1">
          <div className="grid gap-[3px] sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {memories.map((memory) => (
              <button
                key={memory.id}
                type="button"
                onClick={() => openMemoryDetail(memory.id)}
                className="group flex flex-col overflow-hidden rounded-lg border border-neutral-200/80 dark:border-neutral-800 bg-neutral-100 dark:bg-neutral-900/50 text-left transition hover:border-neutral-300 dark:hover:border-neutral-600 hover:bg-neutral-100 dark:hover:bg-neutral-800/80"
              >
                <div className="aspect-[4/3] w-full overflow-hidden bg-neutral-800">
                  <img
                    src={getThumbnailUrl(memory.cover_media_id, "small")}
                    alt=""
                    className="h-full w-full object-cover transition group-hover:scale-105"
                    loading="lazy"
                  />
                </div>
                <div className="p-3">
                  <p className="font-medium text-neutral-100">{memory.title}</p>
                  {memory.subtitle && (
                    <p className="mt-0.5 truncate text-sm text-neutral-500">{memory.subtitle}</p>
                  )}
                  <p className="mt-1 text-sm text-neutral-500">
                    {t("memories.photos", { count: memory.media_count })}
                  </p>
                </div>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
