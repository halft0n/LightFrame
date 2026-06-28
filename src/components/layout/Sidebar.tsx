import { useTranslation } from "@/i18n/useTranslation";
import { setView, useAppStore, type AppView } from "@/store/appStore";
import { NavIcon, type NavIconName } from "./NavIcons";

const NAV_SECTIONS = [
  {
    titleKey: "sidebar.library",
    items: [
      { key: "sidebar.allPhotos", icon: "all" as const, view: "all" as const },
      { key: "sidebar.timeline", icon: "timeline" as const, view: "timeline" as const },
      { key: "sidebar.locations", icon: "locations" as const, view: "locations" as const },
      { key: "sidebar.favorites", icon: "favorites" as const, view: "favorites" as const },
      { key: "sidebar.albums", icon: "albums" as const, view: "albums" as const },
      { key: "sidebar.smartAlbums", icon: "smart-albums" as const, view: "smart-albums" as const },
      { key: "sidebar.memories", icon: "memories" as const, view: "memories" as const },
      { key: "sidebar.people", icon: "people" as const, view: "people" as const },
    ],
  },
  {
    titleKey: "sidebar.tools",
    items: [
      { key: "sidebar.duplicates", icon: "duplicates" as const, view: "duplicates" as const },
      { key: "sidebar.screenshots", icon: "screenshots" as const, view: "screenshots" as const },
      { key: "sidebar.deleted", icon: "deleted" as const, view: "deleted" as const },
    ],
  },
] as const;

function isNavActive(currentView: AppView, itemView: AppView): boolean {
  if (currentView === itemView) return true;
  if (itemView === "albums" && currentView === "album-detail") return true;
  if (itemView === "smart-albums" && currentView === "smart-album-detail") return true;
  if (itemView === "memories" && currentView === "memory-detail") return true;
  if (itemView === "people" && currentView === "person-detail") return true;
  return false;
}

function navButtonClass(isActive: boolean): string {
  const base =
    "sidebar-nav-item w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-[13px] font-medium transition-all duration-200 ease-out";
  if (isActive) {
    return `${base} sidebar-nav-item-active text-[var(--sidebar-active-text)]`;
  }
  return `${base} text-neutral-600 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100`;
}

export function Sidebar() {
  const { t } = useTranslation();
  const { currentView, totalCount } = useAppStore();

  const handleNav = (view: AppView) => {
    setView(view);
  };

  return (
    <aside className="sidebar-glass flex w-[220px] shrink-0 flex-col border-r border-neutral-200/80 dark:border-neutral-800/80">
      <div className="px-4 pb-2 pt-5">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-blue-500 to-indigo-600 shadow-sm">
            <svg viewBox="0 0 24 24" className="h-4 w-4 text-white" fill="currentColor" aria-hidden="true">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" opacity="0.9" />
            </svg>
          </div>
          <div>
            <span className="text-sm font-semibold tracking-tight text-neutral-900 dark:text-neutral-100">
              CatchLight
            </span>
            {totalCount > 0 && (
              <p className="text-[11px] text-neutral-500 dark:text-neutral-500">
                {t("gallery.count", { count: totalCount })}
              </p>
            )}
          </div>
        </div>
      </div>

      <nav className="flex-1 space-y-5 overflow-y-auto px-3 pb-3">
        {NAV_SECTIONS.map((section) => (
          <div key={section.titleKey}>
            <h3 className="sidebar-section-title mb-1.5 px-3 text-[10px] font-semibold uppercase tracking-widest text-neutral-400 dark:text-neutral-500">
              {t(section.titleKey)}
            </h3>
            <ul className="space-y-0.5">
              {section.items.map((item) => (
                <li key={item.view}>
                  <button
                    type="button"
                    onClick={() => handleNav(item.view)}
                    className={navButtonClass(isNavActive(currentView, item.view))}
                  >
                    <NavIcon
                      name={item.icon as NavIconName}
                      className={isNavActive(currentView, item.view) ? "opacity-100" : "opacity-70"}
                    />
                    <span>{t(item.key)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </nav>

      <div className="border-t border-neutral-200/80 px-3 py-3 dark:border-neutral-800/80">
        <button
          type="button"
          onClick={() => handleNav("settings")}
          className={navButtonClass(currentView === "settings")}
        >
          <NavIcon name="settings" className={currentView === "settings" ? "opacity-100" : "opacity-70"} />
          <span>{t("sidebar.settings")}</span>
        </button>
      </div>
    </aside>
  );
}
