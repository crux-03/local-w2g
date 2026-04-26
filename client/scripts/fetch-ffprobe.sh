#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLIENT_DIR="$(dirname "$SCRIPT_DIR")"
BIN_DIR="$CLIENT_DIR/src-tauri/binaries"

# CI passes TARGET; devs auto-detect from rustc
if [[ -z "${TARGET:-}" ]]; then
  TARGET=$(rustc -vV | sed -n 's/host: //p')
fi

EXT=""
[[ "$TARGET" == *windows* ]] && EXT=".exe"
OUT="$BIN_DIR/ffprobe-$TARGET$EXT"

if [[ -f "$OUT" ]]; then
  echo "✓ ffprobe already present at $OUT"
  exit 0
fi

mkdir -p "$BIN_DIR"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# ffbinaries pinned version — bump deliberately, not automatically
FFB_VERSION="6.1"
FFB_BASE="https://github.com/ffbinaries/ffbinaries-prebuilt/releases/download/v${FFB_VERSION}"

echo "Fetching ffprobe for $TARGET..."

case "$TARGET" in
  x86_64-unknown-linux-gnu)
    curl -fL -o "$TMP/ff.zip" "$FFB_BASE/ffprobe-${FFB_VERSION}-linux-64.zip"
    unzip -q "$TMP/ff.zip" -d "$TMP"
    cp "$TMP/ffprobe" "$OUT"
    chmod +x "$OUT"
    ;;
  x86_64-pc-windows-msvc)
    curl -fL -o "$TMP/ff.zip" "$FFB_BASE/ffprobe-${FFB_VERSION}-win-64.zip"
    unzip -q "$TMP/ff.zip" -d "$TMP"
    cp "$TMP/ffprobe.exe" "$OUT"
    ;;
  x86_64-apple-darwin)
    curl -fL -o "$TMP/ff.zip" "$FFB_BASE/ffprobe-${FFB_VERSION}-osx-64.zip"
    unzip -q "$TMP/ff.zip" -d "$TMP"
    cp "$TMP/ffprobe" "$OUT"
    chmod +x "$OUT"
    ;;
  aarch64-apple-darwin)
    # ffbinaries doesn't ship osx-arm64; fall back to osxexperts
    curl -fL -o "$TMP/ff.zip" "https://www.osxexperts.net/ffprobe71arm.zip"
    unzip -q "$TMP/ff.zip" -d "$TMP"
    cp "$TMP/ffprobe" "$OUT"
    chmod +x "$OUT"
    ;;
  *)
    echo "Unsupported target: $TARGET" >&2
    exit 1
    ;;
esac

echo "✓ Installed $OUT ($(du -h "$OUT" | cut -f1))"
