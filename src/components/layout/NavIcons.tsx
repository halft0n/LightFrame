import type { ReactElement, SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const ICON_SIZE = "h-4 w-4 shrink-0";

function cn(...classes: (string | undefined)[]): string {
  return classes.filter(Boolean).join(" ");
}

export function NavIconAllPhotos({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <rect x="3" y="3" width="7" height="7" rx="1.5" />
      <rect x="14" y="3" width="7" height="7" rx="1.5" />
      <rect x="3" y="14" width="7" height="7" rx="1.5" />
      <rect x="14" y="14" width="7" height="7" rx="1.5" />
    </svg>
  );
}

export function NavIconTimeline({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <rect x="3" y="4" width="18" height="18" rx="2" />
      <path d="M16 2v4M8 2v4M3 10h18" strokeLinecap="round" />
    </svg>
  );
}

export function NavIconLocations({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <path d="M12 21s7-4.5 7-11a7 7 0 1 0-14 0c0 6.5 7 11 7 11z" />
      <circle cx="12" cy="10" r="2.5" />
    </svg>
  );
}

export function NavIconFavorites({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <path d="M12 20.5l-1.45-1.32C5.4 14.36 2 11.28 2 7.5 2 5 4 3 6.5 3c1.54 0 3.04.99 3.57 2.36h.86C11.46 3.99 12.96 3 14.5 3 17 3 19 5 19 7.5c0 3.78-3.4 6.86-8.55 11.68L12 20.5z" />
    </svg>
  );
}

export function NavIconVideos({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <rect x="3" y="5" width="18" height="14" rx="2" />
      <path d="M10 9.5v5l4.5-2.5L10 9.5z" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function NavIconAlbums({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <path d="M4 6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v14a1 1 0 0 1-1.447.894L12 17.118l-6.553 3.776A1 1 0 0 1 4 20V6z" />
    </svg>
  );
}

export function NavIconSmartAlbums({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z" />
      <path d="M19 15l.75 2.25L22 18l-2.25.75L19 21l-.75-2.25L16 18l2.25-.75L19 15z" opacity="0.7" />
    </svg>
  );
}

export function NavIconMemories({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3 2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function NavIconPeople({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <circle cx="12" cy="8" r="4" />
      <path d="M4 20c0-4 3.6-6 8-6s8 2 8 6" strokeLinecap="round" />
    </svg>
  );
}

export function NavIconDuplicates({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <rect x="8" y="8" width="12" height="12" rx="2" />
      <path d="M4 16V6a2 2 0 0 1 2-2h10" strokeLinecap="round" />
    </svg>
  );
}

export function NavIconScreenshots({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <rect x="5" y="2" width="14" height="20" rx="2" />
      <path d="M10 18h4" strokeLinecap="round" />
    </svg>
  );
}

export function NavIconDeleted({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <path d="M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function NavIconSettings({ className, ...rest }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" className={cn(ICON_SIZE, className)} {...rest}>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" strokeLinecap="round" />
    </svg>
  );
}

export type NavIconName =
  | "all"
  | "videos"
  | "timeline"
  | "locations"
  | "favorites"
  | "albums"
  | "smart-albums"
  | "memories"
  | "people"
  | "duplicates"
  | "screenshots"
  | "deleted"
  | "settings";

const NAV_ICONS: Record<NavIconName, (props: IconProps) => ReactElement> = {
  all: NavIconAllPhotos,
  videos: NavIconVideos,
  timeline: NavIconTimeline,
  locations: NavIconLocations,
  favorites: NavIconFavorites,
  albums: NavIconAlbums,
  "smart-albums": NavIconSmartAlbums,
  memories: NavIconMemories,
  people: NavIconPeople,
  duplicates: NavIconDuplicates,
  screenshots: NavIconScreenshots,
  deleted: NavIconDeleted,
  settings: NavIconSettings,
};

export function NavIcon({ name, ...props }: { name: NavIconName } & IconProps) {
  const Component = NAV_ICONS[name];
  return <Component {...props} />;
}
