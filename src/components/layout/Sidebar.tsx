import { useTranslation } from "@/i18n/useTranslation";

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

export function Sidebar() {
  const { t } = useTranslation();

  return (
    <aside className="w-56 flex-shrink-0 border-r border-neutral-800 bg-neutral-900/50 flex flex-col">
      <div className="px-4 py-4">
        <span className="text-sm font-bold tracking-wide text-neutral-400 uppercase">
          CatchLight
        </span>
      </div>

      <nav className="flex-1 overflow-y-auto px-2 space-y-4">
        {NAV_SECTIONS.map((section) => (
          <div key={section.titleKey}>
            <h3 className="px-2 mb-1 text-xs font-medium text-neutral-500 uppercase tracking-wider">
              {t(section.titleKey)}
            </h3>
            <ul className="space-y-0.5">
              {section.items.map((item) => (
                <li key={item.view}>
                  <button
                    className="w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm text-neutral-300 hover:bg-neutral-800 hover:text-neutral-100 transition-colors"
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

      <div className="px-2 py-3 border-t border-neutral-800">
        <button className="w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm text-neutral-400 hover:bg-neutral-800 hover:text-neutral-100 transition-colors">
          <span className="text-base">⚙</span>
          <span>{t("sidebar.settings")}</span>
        </button>
      </div>
    </aside>
  );
}
