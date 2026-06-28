# CatchLight / 拾光

[![Build Status](https://github.com/halft0n/CatchLight/actions/workflows/build.yml/badge.svg)](https://github.com/halft0n/CatchLight/actions/workflows/build.yml)
[![Latest Release](https://img.shields.io/github/v/release/halft0n/CatchLight?include_prereleases&label=release)](https://github.com/halft0n/CatchLight/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)](https://github.com/halft0n/CatchLight/releases)
[![Language](https://img.shields.io/badge/backend-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![Framework](https://img.shields.io/badge/framework-Tauri%202-blue?logo=tauri)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

> 拾一束光，留一段时光。 Catch the light, keep the moment.

CatchLight 是一款跨平台（Windows + Linux + macOS）、本地优先的照片查看与管理软件。不复制、不导入，以 Everything 级极速索引就地浏览用户现有文件夹中的照片与视频，并提供去重、截图识别等智能整理能力。

## New in 0.0.5

- **Folder tree** — browse watched folders directly from the sidebar
- **Thumbnail sizes** — Small / Medium / Large presets in the photo grid
- **Search history** — quick access to your last 10 searches
- **Photo rotation** — rotate in the viewer with `R` / `Shift+R`
- **Batch export** — copy selected photos to any folder
- **Smarter screenshot detection** — EXIF rules + visual feature scoring
- **Performance** — 15 new SQL indexes, keyset pagination, SQLite tuning
- **File watcher** — handles deletions (soft-delete) and renames automatically

See [CHANGELOG.md](CHANGELOG.md) for the full release history.

## Features

- **macOS Photos-style UI** — timeline, locations, albums, people, folder tree
- **Lightning-fast indexing** — recursive walkdir + inotify file watching
- **No import/export required** — folders are your library, zero migration cost
- **Non-destructive editor** — curves, levels, selective color adjustments
- **Smart deduplication** — BLAKE3 exact + DHash perceptual matching
- **Screenshot detection** — rule engine with EXIF and visual scoring
- **Map view** — Leaflet map with location clustering
- **Cross-platform** — Windows 10/11, Linux (Ubuntu 22.04+, Fedora 38+), macOS
- **Multi-language** — 简体中文, English
- **Privacy-first** — all local processing, no cloud uploads ([Privacy Policy](docs/PRIVACY.md))

## Keyboard Shortcuts

| Context | Shortcut | Action |
|---------|----------|--------|
| Photo grid | `Escape` | Clear selection |
| Photo grid | `Ctrl`/`Cmd` + click | Toggle item selection |
| Photo grid | `Shift` + click | Range selection |
| Photo viewer | `←` / `→` | Previous / next photo |
| Photo viewer | `R` / `Shift+R` | Rotate clockwise / counter-clockwise |
| Photo viewer | `Escape` | Close viewer |
| Photo viewer | Scroll wheel | Zoom in / out |
| Video player | `Space` | Play / pause |
| Video player | `←` / `→` | Seek −5s / +5s |
| Image editor | `Ctrl+Z` | Undo |
| Image editor | `Ctrl+Shift+Z` | Redo |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19, TypeScript, Vite, TailwindCSS v4 |
| Desktop Shell | Tauri 2.x |
| Backend | Rust (Cargo Workspace) |
| Database | SQLite + FTS5 |
| Mapping | Leaflet (react-leaflet) |
| AI Extension | Python sidecar (optional, local JSON-RPC) |

## Project Structure

```
CatchLight/
├── src/                    # React frontend
├── src-tauri/              # Tauri main crate
├── crates/                 # Rust domain crates
│   ├── catchlight-core/
│   ├── catchlight-db/
│   ├── catchlight-indexer/
│   ├── catchlight-metadata/
│   ├── catchlight-thumbnail/
│   ├── catchlight-dedup/
│   ├── catchlight-ai/
│   ├── catchlight-geo/
│   └── catchlight-video/
├── python/                 # Optional AI sidecar
├── locales/                # i18n translation files
└── docs/                   # Design documents
```

## Development

### Prerequisites

- Node.js >= 22
- pnpm >= 9
- Rust >= 1.77 (via [rustup](https://rustup.rs))
- **Linux:** `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`
- **Optional AI sidecar:** Python 3.10+

### Getting Started

```bash
# Install frontend dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build

# Run tests
pnpm test          # frontend (Vitest)
cargo test         # Rust workspace
```

## Documentation

| Doc | Description |
|-----|-------------|
| [Changelog](CHANGELOG.md) | Release history |
| [Privacy Policy](docs/PRIVACY.md) | Data handling and privacy |
| [Research Report](docs/0-research-report.md) | Technology research and feasibility analysis |
| [Tech Stack Decision](docs/1-tech-stack-decision.md) | Hybrid architecture decision rationale |
| [Requirements](docs/2-requirements.md) | Functional and non-functional requirements |
| [Architecture](docs/3-architecture.md) | System architecture design |
| [Detailed Design](docs/4-detailed-design.md) | Implementation-level module design |
| [Development Plan](docs/5-development-plan.md) | 24-week phased development roadmap |

## Acknowledgements

CatchLight 的诞生离不开以下优秀的开源项目：

| Project | Usage |
|---------|-------|
| [Tauri](https://tauri.app) | Cross-platform desktop framework |
| [React](https://react.dev) | Frontend UI library |
| [Vite](https://vite.dev) | Frontend build tool |
| [Tailwind CSS](https://tailwindcss.com) | Utility-first CSS framework |
| [SQLite](https://sqlite.org) | Embedded database engine |
| [rusqlite](https://github.com/rusqlite/rusqlite) | SQLite bindings for Rust |
| [image](https://github.com/image-rs/image) | Rust image processing library |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | Fast cryptographic hash for deduplication |
| [kamadak-exif](https://github.com/kamadak/exif-rs) | EXIF metadata extraction |
| [reverse_geocoder](https://github.com/gx0r/rrgeo) | Offline reverse geocoding |
| [Leaflet](https://leafletjs.com) | Interactive map for location browsing |
| [iPhotron](https://github.com/OliverZhaohaibin/iPhotron-LocalPhotoAlbumManager) | UI/UX design inspiration |
| [@fontsource/inter](https://fontsource.org/fonts/inter) | Self-hosted Inter font |
| [@tanstack/react-virtual](https://tanstack.com/virtual) | Virtual scrolling for large lists |

## Star History

<p align="center">
  <a href="https://www.star-history.com/#halft0n/CatchLight&type=Date">
    <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=halft0n/CatchLight&type=Date" />
  </a>
</p>

## License

MIT License. See [LICENSE](LICENSE) for details.
