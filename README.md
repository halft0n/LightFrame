# CatchLight / 拾光

[![Build Status](https://github.com/halft0n/CatchLight/actions/workflows/build.yml/badge.svg)](https://github.com/halft0n/CatchLight/actions/workflows/build.yml)
[![Latest Release](https://img.shields.io/github/v/release/halft0n/CatchLight?include_prereleases&label=release)](https://github.com/halft0n/CatchLight/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)](https://github.com/halft0n/CatchLight/releases)
[![Language](https://img.shields.io/badge/backend-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![Framework](https://img.shields.io/badge/framework-Tauri%202-blue?logo=tauri)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

> 拾一束光，留一段时光。 Catch the light, keep the moment.

CatchLight 是一款跨平台（Windows + Linux + macOS）、本地优先的照片查看与管理软件。不复制、不导入，以 Everything 级极速索引就地浏览用户现有文件夹中的照片与视频，并提供去重、截图识别等智能整理能力。

## Features

- **macOS Photos-style UI** — 时间线、地点、相簿、人物等多维度浏览
- **Lightning-fast indexing** — Windows NTFS MFT/USN 秒级索引，Linux inotify 实时监听
- **No import/export** — 文件夹即图库，零迁移成本
- **Smart deduplication** — BLAKE3 精确去重 + DHash/PHash 感知去重 + CLIP 语义去重
- **Screenshot detection** — 多层 AI 自动识别截图并分类（代码/聊天/文档/游戏）
- **Cross-platform** — Windows 10/11 + Linux (Ubuntu 22.04+, Fedora 38+)
- **Multi-language** — 简体中文、English，可扩展
- **Privacy-first** — 全部本地处理，照片不上传任何服务器

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19, TypeScript, Vite, TailwindCSS v4, shadcn/ui |
| Desktop Shell | Tauri 2.x |
| Backend | Rust (Cargo Workspace) |
| AI Extension | Python sidecar (optional, JSON-RPC) |
| Database | SQLite + FTS5 |
| Mapping | Leaflet / MapLibre GL |

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
├── locales/                # i18n translation files
├── catchlight-ai-py/       # Python AI extension (optional)
└── docs/                   # Design documents
```

## Development

### Prerequisites

- Node.js >= 20
- pnpm >= 9
- Rust >= 1.77 (via rustup)
- Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`

### Getting Started

```bash
# Install frontend dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

## Documentation

| Doc | Description |
|-----|-------------|
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
