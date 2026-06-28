import { useTranslation } from "@/i18n/useTranslation";
import { setView, useAppStore, type AppView } from "@/store/appStore";

const NAV_SECTIONS = [
  {
    titleKey: "sidebar.library",
    items: [
      { key: "sidebar.allPhotos", icon: "🖼", view: "all" as const },
      { key: "sidebar.timeline", icon: "📅", view: "timeline" as const },
      { key: "sidebar.locations", icon: "📍", view: "locations" as const },
      { key: "sidebar.people", icon: "👤", view: "people" as const },
    ],
  },
  {
    titleKey: "sidebar.tools",
    items: [
      { key: "sidebar.duplicates", icon: "🔍", view: "duplicates" as const },
      { key: "sidebar.screenshots", icon: "📱", view: "screenshots" as const },
    ],
  },
] as const;

function navButtonClass(isActive: boolean): string {
  const base =
    "w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors";
  if (isActive) {
    return `${base} bg-neutral-800 text-neutral-100 font-medium`;
  }
  return `${base} text-neutral-300 hover:bg-neutral-800 hover:text-neutral-100`;
}

export function Sidebar() {
  const { t } = useTranslation();
  const { currentView, totalCount } = useAppStore();

  const handleNav = (view: AppView) => {
    setView(view);
  };

  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-neutral-800 bg-neutral-900/50">
      <div className="px-4 py-4">
        <span className="text-sm font-bold uppercase tracking-wide text-neutral-400">
          CatchLight
        </span>
        {totalCount > 0 && (
          <p className="mt-1 text-xs text-neutral-500">
            {t("gallery.count", { count: totalCount })}
          </p>
        )}
      </div>

      <nav className="flex-1 space-y-4 overflow-y-auto px-2">
        {NAV_SECTIONS.map((section) => (
          <div key={section.titleKey}>
            <h3 className="mb-1 px-2 text-xs font-medium uppercase tracking-wider text-neutral-500">
              {t(section.titleKey)}
            </h3>
            <ul className="space-y-0.5">
              {section.items.map((item) => (
                <li key={item.view}>
                  <button
                    type="button"
                    onClick={() => handleNav(item.view)}
                    className={navButtonClass(currentView === item.view)}
                  >
                    <span className="text-base">{item.icon}</span>
                    <span>{t(item.key)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </nav>

      <div className="border-t border-neutral-800 px-2 py-3">
        <button
          type="button"
          onClick={() => handleNav("settings")}
          className={navButtonClass(currentView === "settings")}
        >
          <span className="text-base">⚙</span>
          <span>{t("sidebar.settings")}</span>
        </button>
      </div>
    </aside>
  );
}
