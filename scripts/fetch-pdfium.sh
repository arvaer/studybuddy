#!/usr/bin/env bash
# Fetch the prebuilt PDFium library the backend loads at run time for PDF
# text extraction (infra::pdf). From bblanchon/pdfium-binaries, the builds
# pdfium-render documents. Lands in backend/lib/ (gitignored); PDFIUM_DIR
# points the backend elsewhere.
#
#   scripts/fetch-pdfium.sh            # this machine's platform
#   PDFIUM_PLATFORM=linux-x64 scripts/fetch-pdfium.sh
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest="${PDFIUM_DIR:-$root/backend/lib}"
if [ -z "${PDFIUM_PLATFORM:-}" ]; then
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) PDFIUM_PLATFORM=mac-arm64 ;;
    Darwin-x86_64) PDFIUM_PLATFORM=mac-x64 ;;
    Linux-x86_64) PDFIUM_PLATFORM=linux-x64 ;;
    Linux-aarch64) PDFIUM_PLATFORM=linux-arm64 ;;
    *) echo "set PDFIUM_PLATFORM for $(uname -s)-$(uname -m)" >&2; exit 1 ;;
  esac
fi
version="${PDFIUM_VERSION:-latest}"
if [ "$version" = latest ]; then
  url="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-$PDFIUM_PLATFORM.tgz"
else
  url="https://github.com/bblanchon/pdfium-binaries/releases/download/$version/pdfium-$PDFIUM_PLATFORM.tgz"
fi
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
echo "fetching $url"
curl -fsSL "$url" -o "$tmp/pdfium.tgz"
mkdir -p "$dest"
tar -xzf "$tmp/pdfium.tgz" -C "$tmp"
cp "$tmp"/lib/libpdfium.* "$dest"/
cp "$tmp/VERSION" "$dest/PDFIUM_VERSION" 2>/dev/null || true
ls -la "$dest"
