import { useCallback, useEffect, useState } from "react";
import {
  generateMemories,
  getOnThisDay,
  getThumbnailUrl,
  listMemories,
  type MediaItem,
  type Memory,
} from "@/lib/tauri";
import { openMemoryDetail, openViewer } from "@/store/appStore";
import { useTranslation } from "@/i18n/useTranslation";

export function MemoriesView() {
  const { t } = useTranslation();
  const [memories, setMemories] = useState<Memory[]>([]);
  const [onThisDay, setOnThisDay] = useState<MediaItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);

  const loadMemories = useCallback(async () => {
    setLoading(true);
    try {
      const [memoryList, todayItems] = await Promise.all([
        listMemories(),
        getOnThisDay(12),
      ]);
      setMemories(memoryList);
      setOnThisDay(todayItems);
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
        <h2 className="text-sm font-medium text-neutral-200">
          {t("memories.title")}
        </h2>
        <button
          type="button"
          disabled={generating}
          onClick={() => void handleGenerate()}
          className="rounded-md bg-blue-600 px-3 py-1.5 text-sm text-white transition hover:bg-blue-500 disabled:opacity-50"
        >
          {t("memories.generate")}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-1 py-1">
        {onThisDay.length > 0 && (
          <section className="mb-4 px-3 pt-3">
            <div className="overflow-hidden rounded-xl border border-neutral-200/80 bg-gradient-to-r from-blue-950/40 to-neutral-900/60 dark:border-neutral-700">
              <div className="border-b border-white/10 px-4 py-3">
                <h3 className="text-base font-semibold text-neutral-100">
                  {t("memories.onThisDay")}
                </h3>
                <p className="mt-0.5 text-sm text-neutral-400">
                  {t("memories.onThisDayHint")}
                </p>
              </div>
              <div className="flex gap-2 overflow-x-auto p-4">
                {onThisDay.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => openViewer(item.id)}
                    className="group h-24 w-24 shrink-0 overflow-hidden rounded-lg ring-1 ring-white/10 transition hover:ring-blue-500"
                  >
                    <img
                      src={getThumbnailUrl(item.id, "small")}
                      alt={item.filename}
                      className="h-full w-full object-cover transition group-hover:scale-105"
                      loading="lazy"
                    />
                  </button>
                ))}
              </div>
            </div>
          </section>
        )}

        {memories.length === 0 ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-2 py-16 text-neutral-500">
            <div className="text-5xl">🕰</div>
            <p className="text-lg">{t("memories.empty")}</p>
            <p className="text-sm text-neutral-600">
              {t("memories.emptyHint")}
            </p>
          </div>
        ) : (
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
                    <p className="mt-0.5 truncate text-sm text-neutral-500">
                      {memory.subtitle}
                    </p>
                  )}
                  <p className="mt-1 text-sm text-neutral-500">
                    {t("memories.photos", { count: memory.media_count })}
                  </p>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
