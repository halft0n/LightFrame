import { useState } from "react";
import { useTranslation } from "@/i18n/useTranslation";
import { setView, useAppStore, type AppView } from "@/store/appStore";
import { NavIcon, type NavIconName } from "./NavIcons";

const LIBRARY_ITEMS: SidebarItem[] = [
  { key: "sidebar.allPhotos", icon: "all", view: "all" },
  { key: "sidebar.videos", icon: "videos", view: "videos" },
  { key: "sidebar.favorites", icon: "favorites", view: "favorites" },
  { key: "sidebar.locations", icon: "locations", view: "locations" },
  { key: "sidebar.people", icon: "people", view: "people" },
];

const ALBUM_ITEMS: SidebarItem[] = [
  { key: "sidebar.albums", icon: "albums", view: "albums" },
  { key: "sidebar.smartAlbums", icon: "smart-albums", view: "smart-albums" },
  { key: "sidebar.duplicates", icon: "duplicates", view: "duplicates" },
  { key: "sidebar.screenshots", icon: "screenshots", view: "screenshots" },
];

interface SidebarItem {
  key: string;
  icon: NavIconName;
  view: AppView;
}

function isNavActive(currentView: AppView, itemView: AppView): boolean {
  if (currentView === itemView) return true;
  if (itemView === "albums" && currentView === "album-detail") return true;
  if (itemView === "smart-albums" && currentView === "smart-album-detail") return true;
  if (itemView === "people" && currentView === "person-detail") return true;
  return false;
}

function navItemClass(isActive: boolean): string {
  const base =
    "sidebar-nav-item w-full flex items-center gap-2 pl-6 pr-2 py-[5px] rounded-md text-[13px] transition-all duration-150";
  if (isActive) {
    return `${base} sidebar-nav-item-active font-medium text-[var(--sidebar-active-text)]`;
  }
  return `${base} text-neutral-700 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100`;
}

function TreeHeader({
  label,
  icon,
  expanded,
  onToggle,
}: {
  label: string;
  icon: React.ReactNode;
  expanded: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onToggle}
      className="flex w-full items-center gap-1.5 px-2 py-[5px] text-[13px] font-semibold text-neutral-900 transition-colors hover:text-neutral-700 dark:text-neutral-200 dark:hover:text-neutral-100"
    >
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.5"
        className={`h-3 w-3 shrink-0 text-neutral-400 transition-transform duration-150 ${expanded ? "rotate-90" : ""}`}
        aria-hidden="true"
      >
        <path d="M9 6l6 6-6 6" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
      {icon}
      <span>{label}</span>
    </button>
  );
}

function NavItem({
  item,
  currentView,
  onNav,
}: {
  item: SidebarItem;
  currentView: AppView;
  onNav: (view: AppView) => void;
}) {
  const { t } = useTranslation();
  const active = isNavActive(currentView, item.view);

  return (
    <li>
      <button type="button" onClick={() => onNav(item.view)} className={navItemClass(active)}>
        <NavIcon name={item.icon} className={active ? "opacity-100" : "opacity-60"} />
        <span>{t(item.key)}</span>
      </button>
    </li>
  );
}

export function Sidebar() {
  const { t } = useTranslation();
  const { currentView } = useAppStore();
  const [libraryExpanded, setLibraryExpanded] = useState(true);
  const [albumsExpanded, setAlbumsExpanded] = useState(true);

  const handleNav = (view: AppView) => {
    setView(view);
  };

  return (
    <aside className="sidebar-glass flex w-[180px] shrink-0 flex-col border-r border-neutral-200/60 dark:border-neutral-800/60">
      <div className="px-3 pb-1 pt-3">
        <span className="text-[11px] font-bold uppercase tracking-widest text-neutral-400 dark:text-neutral-500">
          {t("sidebar.basicLibrary")}
        </span>
      </div>

      <nav className="flex-1 space-y-1 overflow-y-auto px-1.5 pb-2 pt-1">
        <div>
          <TreeHeader
            label={t("sidebar.basicLibrary")}
            icon={
              <svg viewBox="0 0 24 24" className="h-4 w-4 shrink-0 text-blue-500" fill="currentColor" aria-hidden="true">
                <rect x="3" y="3" width="7" height="7" rx="1.5" opacity="0.8" />
                <rect x="14" y="3" width="7" height="7" rx="1.5" opacity="0.6" />
                <rect x="3" y="14" width="7" height="7" rx="1.5" opacity="0.6" />
                <rect x="14" y="14" width="7" height="7" rx="1.5" opacity="0.4" />
              </svg>
            }
            expanded={libraryExpanded}
            onToggle={() => setLibraryExpanded((v) => !v)}
          />
          {libraryExpanded && (
            <ul className="space-y-px">
              {LIBRARY_ITEMS.map((item) => (
                <NavItem key={item.view} item={item} currentView={currentView} onNav={handleNav} />
              ))}
            </ul>
          )}
        </div>

        <div>
          <TreeHeader
            label={t("sidebar.albumsSection")}
            icon={
              <svg viewBox="0 0 24 24" className="h-4 w-4 shrink-0 text-neutral-500" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden="true">
                <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
              </svg>
            }
            expanded={albumsExpanded}
            onToggle={() => setAlbumsExpanded((v) => !v)}
          />
          {albumsExpanded && (
            <ul className="space-y-px">
              {ALBUM_ITEMS.map((item) => (
                <NavItem key={item.view} item={item} currentView={currentView} onNav={handleNav} />
              ))}
            </ul>
          )}
        </div>
      </nav>

      <div className="space-y-px border-t border-neutral-200/60 px-1.5 py-1.5 dark:border-neutral-800/60">
        <button
          type="button"
          onClick={() => handleNav("deleted")}
          className={navItemClass(isNavActive(currentView, "deleted"))}
        >
          <NavIcon name="deleted" className={isNavActive(currentView, "deleted") ? "opacity-100" : "opacity-60"} />
          <span>{t("sidebar.deleted")}</span>
        </button>
        <button
          type="button"
          onClick={() => handleNav("settings")}
          className={navItemClass(currentView === "settings")}
        >
          <NavIcon name="settings" className={currentView === "settings" ? "opacity-100" : "opacity-60"} />
          <span>{t("sidebar.settings")}</span>
        </button>
      </div>
    </aside>
  );
}
