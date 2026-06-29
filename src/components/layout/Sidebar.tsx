import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "@/i18n/useTranslation";
import { addToAlbum, listAlbums, type Album } from "@/lib/tauri";
import { parseDragMediaIds } from "@/lib/dragMedia";
import { navigate, openAlbumDetail, useAppStore, type AppView } from "@/store/appStore";
import { NavIcon, type NavIconName } from "./NavIcons";

const LIBRARY_ITEMS: SidebarItem[] = [
  { key: "sidebar.allPhotos", icon: "all", view: "all" },
  { key: "sidebar.videos", icon: "videos", view: "videos" },
  { key: "sidebar.timeline", icon: "timeline", view: "timeline" },
  { key: "sidebar.favorites", icon: "favorites", view: "favorites" },
  { key: "sidebar.locations", icon: "locations", view: "locations" },
  { key: "sidebar.people", icon: "people", view: "people" },
];

const ALBUM_ITEMS: SidebarItem[] = [
  { key: "sidebar.albums", icon: "albums", view: "albums" },
  { key: "sidebar.smartAlbums", icon: "smart-albums", view: "smart-albums" },
  { key: "sidebar.memories", icon: "memories", view: "memories" },
  { key: "sidebar.duplicates", icon: "duplicates", view: "duplicates" },
  { key: "sidebar.screenshots", icon: "screenshots", view: "screenshots" },
];

interface SidebarItem {
  key: string;
  icon: NavIconName;
  view: AppView;
}

interface SidebarProps {
  isMobile?: boolean;
  mobileOpen?: boolean;
  onMobileClose?: () => void;
}

function folderDisplayName(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function FolderIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      className={`h-4 w-4 shrink-0 ${className ?? ""}`}
      aria-hidden="true"
    >
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
    </svg>
  );
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
    "sidebar-nav-item w-full flex items-center gap-2 pl-6 pr-2 py-[5px] rounded-md text-[13px]";
  if (isActive) {
    return `${base} sidebar-nav-item-active font-medium text-[var(--sidebar-active-text)]`;
  }
  return `${base} text-neutral-700 hover:text-neutral-900 dark:text-neutral-400 dark:hover:text-neutral-100`;
}

function CollapsibleSection({
  id,
  label,
  icon,
  expanded,
  onToggle,
  children,
}: {
  id: string;
  label: string;
  icon: React.ReactNode;
  expanded: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}) {
  return (
    <div>
      <button
        type="button"
        id={`${id}-header`}
        aria-expanded={expanded}
        aria-controls={`${id}-panel`}
        onClick={onToggle}
        aria-label={label}
        className="flex w-full items-center gap-1.5 px-2 py-[5px] text-[13px] font-semibold text-neutral-900 transition-colors hover:text-neutral-700 dark:text-neutral-200 dark:hover:text-neutral-100"
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          className={`h-3 w-3 shrink-0 text-neutral-400 transition-transform duration-200 ${expanded ? "rotate-90" : ""}`}
          aria-hidden="true"
        >
          <path d="M9 6l6 6-6 6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        {icon}
        <span>{label}</span>
      </button>
      <div
        id={`${id}-panel`}
        role="region"
        aria-labelledby={`${id}-header`}
        className={`sidebar-section-content ${expanded ? "sidebar-section-content--expanded" : ""}`}
      >
        <div className="sidebar-section-inner">{children}</div>
      </div>
    </div>
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
      <button
        type="button"
        onClick={() => onNav(item.view)}
        className={navItemClass(active)}
        aria-current={active ? "page" : undefined}
      >
        <NavIcon name={item.icon} className={active ? "opacity-100" : "opacity-60"} />
        <span>{t(item.key)}</span>
      </button>
    </li>
  );
}

function AlbumDropItem({
  album,
  currentView,
  selectedAlbumId,
  dragOverAlbumId,
  onDragOver,
  onDragLeave,
  onDrop,
}: {
  album: Album;
  currentView: AppView;
  selectedAlbumId: number | null;
  dragOverAlbumId: number | null;
  onDragOver: (albumId: number, e: React.DragEvent) => void;
  onDragLeave: () => void;
  onDrop: (albumId: number, e: React.DragEvent) => void;
}) {
  const active = currentView === "album-detail" && selectedAlbumId === album.id;
  const isDragOver = dragOverAlbumId === album.id;

  return (
    <li>
      <button
        type="button"
        onClick={() => openAlbumDetail(album.id)}
        onDragOver={(e) => onDragOver(album.id, e)}
        onDragLeave={onDragLeave}
        onDrop={(e) => onDrop(album.id, e)}
        className={`${navItemClass(active)} ${isDragOver ? "bg-blue-500/20 ring-1 ring-blue-500" : ""}`}
        title={album.name}
        aria-current={active ? "page" : undefined}
      >
        <NavIcon name="albums" className={active ? "opacity-100" : "opacity-60"} />
        <span className="truncate">{album.name}</span>
        <span className="ml-auto text-[11px] tabular-nums text-neutral-400">{album.media_count}</span>
      </button>
    </li>
  );
}

export function Sidebar({ isMobile = false, mobileOpen = false, onMobileClose }: SidebarProps) {
  const { t } = useTranslation();
  const { currentView, watchedFolders, selectedFolderId, selectedAlbumId } = useAppStore();
  const [libraryExpanded, setLibraryExpanded] = useState(true);
  const [foldersExpanded, setFoldersExpanded] = useState(true);
  const [albumsExpanded, setAlbumsExpanded] = useState(true);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [dragOverAlbumId, setDragOverAlbumId] = useState<number | null>(null);
  const [closing, setClosing] = useState(false);

  useEffect(() => {
    void listAlbums()
      .then(setAlbums)
      .catch(() => setAlbums([]));
  }, []);

  useEffect(() => {
    if (mobileOpen) setClosing(false);
  }, [mobileOpen]);

  const handleNav = (view: AppView) => {
    navigate(view);
    onMobileClose?.();
  };

  const handleMobileClose = () => {
    if (!isMobile || !onMobileClose) return;
    setClosing(true);
    window.setTimeout(() => {
      onMobileClose();
      setClosing(false);
    }, 200);
  };

  const handleAlbumDragOver = useCallback((albumId: number, e: React.DragEvent) => {
    if (![...e.dataTransfer.types].includes("application/x-lightframe-media-ids")) {
      return;
    }
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setDragOverAlbumId(albumId);
  }, []);

  const handleAlbumDragLeave = useCallback(() => {
    setDragOverAlbumId(null);
  }, []);

  const handleAlbumDrop = useCallback(async (albumId: number, e: React.DragEvent) => {
    e.preventDefault();
    setDragOverAlbumId(null);
    const mediaIds = parseDragMediaIds(e.dataTransfer);
    if (mediaIds.length === 0) return;
    try {
      await addToAlbum(albumId, mediaIds);
    } catch (err) {
      console.error("Failed to add photos to album:", err);
    }
  }, []);

  if (isMobile && !mobileOpen) {
    return null;
  }

  const sidebarClass = isMobile
    ? `sidebar-mobile sidebar-glass flex w-[240px] shrink-0 flex-col border-r border-neutral-200/60 dark:border-neutral-800/60 ${
        closing ? "sidebar-mobile--closing" : ""
      }`
    : "sidebar-glass sidebar-desktop hidden w-[180px] shrink-0 flex-col border-r border-neutral-200/60 md:flex dark:border-neutral-800/60";

  return (
    <aside className={sidebarClass} aria-label={t("sidebar.navigation")}>
      {isMobile && (
        <div className="flex items-center justify-between border-b border-neutral-200/60 px-3 py-2 dark:border-neutral-800/60">
          <span className="text-sm font-semibold text-neutral-900 dark:text-neutral-100">
            {t("app.title")}
          </span>
          <button
            type="button"
            onClick={handleMobileClose}
            className="rounded-md p-1.5 text-neutral-500 transition hover:bg-neutral-200/60 hover:text-neutral-800 dark:hover:bg-neutral-800 dark:hover:text-neutral-200"
            aria-label={t("sidebar.closeSidebar")}
          >
            <svg viewBox="0 0 24 24" className="h-5 w-5" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
              <path d="M6 6l12 12M18 6L6 18" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      )}

      {!isMobile && (
        <div className="px-3 pb-1 pt-3">
          <span className="text-[11px] font-bold uppercase tracking-widest text-neutral-400 dark:text-neutral-500">
            {t("sidebar.basicLibrary")}
          </span>
        </div>
      )}

      <nav role="navigation" aria-label={t("sidebar.navigation")} className="flex-1 space-y-1 overflow-y-auto px-1.5 pb-2 pt-1">
        <CollapsibleSection
          id="sidebar-library"
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
        >
          <ul className="space-y-px pt-px">
            {LIBRARY_ITEMS.map((item) => (
              <NavItem key={item.view} item={item} currentView={currentView} onNav={handleNav} />
            ))}
          </ul>
        </CollapsibleSection>

        {watchedFolders.length > 0 && (
          <CollapsibleSection
            id="sidebar-folders"
            label={t("sidebar.folders")}
            icon={<FolderIcon className="text-amber-500" />}
            expanded={foldersExpanded}
            onToggle={() => setFoldersExpanded((v) => !v)}
          >
            <ul className="space-y-px pt-px">
              {watchedFolders.map((folder) => {
                const active = currentView === "folder" && selectedFolderId === folder.id;
                return (
                  <li key={folder.id}>
                    <button
                      type="button"
                      onClick={() => {
                        navigate("folder", {
                          folderId: folder.id,
                          folderPath: folder.path,
                        });
                        onMobileClose?.();
                      }}
                      className={navItemClass(active)}
                      title={folder.path}
                      aria-current={active ? "page" : undefined}
                    >
                      <FolderIcon className={active ? "opacity-100" : "opacity-60"} />
                      <span className="truncate">{folderDisplayName(folder.path)}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </CollapsibleSection>
        )}

        <CollapsibleSection
          id="sidebar-albums"
          label={t("sidebar.albumsSection")}
          icon={
            <svg viewBox="0 0 24 24" className="h-4 w-4 shrink-0 text-neutral-500" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden="true">
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
            </svg>
          }
          expanded={albumsExpanded}
          onToggle={() => setAlbumsExpanded((v) => !v)}
        >
          <ul className="space-y-px pt-px">
            {ALBUM_ITEMS.map((item) => (
              <NavItem key={item.view} item={item} currentView={currentView} onNav={handleNav} />
            ))}
            {albums.length > 0 && (
              <>
                <li className="px-6 pt-2 pb-0.5">
                  <span className="text-[11px] font-medium uppercase tracking-wide text-neutral-400 dark:text-neutral-500">
                    {t("sidebar.myAlbums")}
                  </span>
                </li>
                {albums.map((album) => (
                  <AlbumDropItem
                    key={album.id}
                    album={album}
                    currentView={currentView}
                    selectedAlbumId={selectedAlbumId}
                    dragOverAlbumId={dragOverAlbumId}
                    onDragOver={handleAlbumDragOver}
                    onDragLeave={handleAlbumDragLeave}
                    onDrop={handleAlbumDrop}
                  />
                ))}
              </>
            )}
          </ul>
        </CollapsibleSection>
      </nav>

      <div className="space-y-px border-t border-neutral-200/60 px-1.5 py-1.5 dark:border-neutral-800/60">
        <button
          type="button"
          onClick={() => handleNav("deleted")}
          className={navItemClass(isNavActive(currentView, "deleted"))}
          aria-current={isNavActive(currentView, "deleted") ? "page" : undefined}
        >
          <NavIcon name="deleted" className={isNavActive(currentView, "deleted") ? "opacity-100" : "opacity-60"} />
          <span>{t("sidebar.deleted")}</span>
        </button>
        <button
          type="button"
          onClick={() => handleNav("settings")}
          className={navItemClass(currentView === "settings")}
          aria-current={currentView === "settings" ? "page" : undefined}
        >
          <NavIcon name="settings" className={currentView === "settings" ? "opacity-100" : "opacity-60"} />
          <span>{t("sidebar.settings")}</span>
        </button>
      </div>
    </aside>
  );
}
