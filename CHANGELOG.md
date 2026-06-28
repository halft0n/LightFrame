# Changelog

All notable changes to CatchLight will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- (nothing yet)

## [0.0.5] - 2026-06-28

### Added
- Folder tree browsing in sidebar
- Thumbnail size control (3 presets: Small/Medium/Large)
- Search history (last 10 searches)
- Photo viewer rotation (`R` / `Shift+R` shortcuts)
- Batch export to folder
- Python AI sidecar JSON-RPC framework
- Enhanced screenshot detection with EXIF rules and visual feature scoring
- SQL performance indexes (15 new indexes)
- Keyset pagination for efficient large dataset browsing
- SQLite PRAGMA optimizations (64MB cache, 256MB mmap)
- Watcher now handles file deletions (soft-delete) and renames (path update)

### Fixed
- `original://` protocol security: path allowlist based on watched folders
- `get_media_by_id` now filters out soft-deleted items
- Permanent delete now actually removes files from disk + thumbnail cache
- Removed double disk-delete between repo and commands layer

### Security
- Added path validation for `original://` protocol to prevent arbitrary file access

## [0.0.4] - 2026-06-28

### Added
- Complete non-destructive image editor with curves, levels, selective color
- `is_favorite` backend command
- Frontend error handling with try/catch on all IPC calls
- Permanent deletion confirmation dialog
- Light/dark theme contrast improvements

### Fixed
- Range selection in VideosView using correct context items
- PersonDetailView pagination logic
- Dialog save permission for image export
- Invalid HTML nesting in album/smart album list views
- AlbumDetailView infinite scrolling
- ImageEditor export state management
- Timer memory leak in ImageEditor
- Deep merge for nested edit parameters
- Theme settings subtitle text

## [0.0.3] - 2026-06-28

### Added
- Initial release with core features
- File indexing with walkdir + inotify watching
- Three-level thumbnail system (micro/small/large) with `thumb://` protocol
- Virtual scrolling photo grid with multi-select
- Timeline view with date grouping
- Full-screen photo viewer with zoom/pan
- BLAKE3 exact deduplication
- DHash perceptual deduplication
- FTS5 full-text search
- Leaflet map view with location clustering
- Album management (CRUD + media association)
- Smart albums with rule-based filtering
- Memories generation (date+location clustering)
- Favorites and soft-delete with 30-day cleanup
- Batch operations (delete, favorite, add to album, restore)
- Video playback support with FFmpeg thumbnails
- Light/dark theme with system preference
- Chinese/English internationalization
