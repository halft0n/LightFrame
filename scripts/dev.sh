#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# --- WSL2 Display ---
if grep -qi microsoft /proc/version 2>/dev/null; then
    export DISPLAY="${DISPLAY:-:0}"
    export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"

    # GPU: disable compositing if DRI not accessible
    if [ ! -r /dev/dri/renderD128 ] 2>/dev/null; then
        echo "[dev] GPU not accessible, disabling compositing"
        export WEBKIT_DISABLE_COMPOSITING_MODE=1
    fi

    # Proxy (if configured on host)
    if [ -n "${http_proxy:-}" ]; then
        echo "[dev] Using proxy: $http_proxy"
    fi
fi

# --- WebKitGTK tweaks ---
export GDK_BACKEND="${GDK_BACKEND:-x11}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"

echo "============================================"
echo "  CatchLight Dev Server"
echo "  DISPLAY=$DISPLAY"
echo "  GDK_BACKEND=$GDK_BACKEND"
echo "============================================"

MODE="${1:-tauri}"

tauri_cmd() {
    if command -v cargo-tauri &>/dev/null; then
        cargo tauri "$@"
    elif [ -x "$PROJECT_DIR/node_modules/.bin/tauri" ]; then
        npx tauri "$@"
    else
        echo "[dev] Error: tauri-cli not found."
        echo "  Install via: cargo install tauri-cli --version '^2'"
        echo "  Or:          pnpm add -D @tauri-apps/cli"
        exit 1
    fi
}

case "$MODE" in
    tauri)
        echo "[dev] Starting Tauri dev (desktop app + Vite)..."
        tauri_cmd dev
        ;;
    web)
        echo "[dev] Starting Vite dev server only (web preview)..."
        pnpm dev
        ;;
    build)
        echo "[dev] Building production release..."
        tauri_cmd build
        ;;
    *)
        echo "Usage: $0 [tauri|web|build]"
        echo "  tauri  - Start Tauri desktop app with Vite (default)"
        echo "  web    - Start Vite dev server only"
        echo "  build  - Build production release"
        exit 1
        ;;
esac
