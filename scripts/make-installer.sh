#!/bin/bash
# Build Volt.app (with AVR tools) and a double-click installer: dist/Volt.pkg
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
VERSION="${VERSION:-0.1.0}"

echo "=== 1. AVR compiler (ships in the installer) ==="
"$ROOT/scripts/fetch-avr-tools.sh"

echo "=== 2. Volt.app ==="
"$ROOT/scripts/package-macos.sh"

APP="$ROOT/dist/Volt.app"
if [[ -d "$ROOT/tools/avr" ]]; then
  echo "Bundling AVR tools into the app…"
  mkdir -p "$APP/Contents/Resources/tools"
  rm -rf "$APP/Contents/Resources/tools/avr"
  cp -R "$ROOT/tools/avr" "$APP/Contents/Resources/tools/avr"
  find "$APP/Contents/Resources/tools" \( -name '.DS_Store' -o -name '._*' \) -delete
  if command -v codesign >/dev/null; then
    codesign --force --deep --sign - "$APP" >/dev/null
  fi
fi

echo "=== 3. macOS installer package ==="
PKGROOT="$ROOT/dist/pkgroot"
rm -rf "$PKGROOT"
mkdir -p "$PKGROOT"
cp -R "$APP" "$PKGROOT/Volt.app"
SCRIPTS="$ROOT/scripts/macos/pkg-scripts"
chmod +x "$SCRIPTS/postinstall" 2>/dev/null || true
pkgbuild \
  --root "$PKGROOT" \
  --identifier dev.volt.ide \
  --version "$VERSION" \
  --install-location /Applications \
  --scripts "$SCRIPTS" \
  "$ROOT/dist/Volt.pkg"

echo
echo "Installer: $ROOT/dist/Volt.pkg"
echo "App:       $APP"
echo "Open the pkg, or:  open \"$ROOT/dist/Volt.pkg\""
echo "AVR gcc is inside the app. Pico/ESP: Get compiler in the IDE."
