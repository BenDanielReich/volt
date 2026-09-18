#!/bin/bash
# Build Volt.app (native WKWebView IDE) into dist/Volt.app
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "Building Volt (macOS app)…"
cargo build --release --features app --bin volt --bin voltc

APP="$ROOT/dist/Volt.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$ROOT/target/release/volt" "$APP/Contents/MacOS/Volt"
cp "$ROOT/target/release/voltc" "$APP/Contents/MacOS/voltc"
chmod +x "$APP/Contents/MacOS/Volt" "$APP/Contents/MacOS/voltc"

cp "$ROOT/ide/macos/Info.plist" "$APP/Contents/Info.plist"
echo -n "APPL????" > "$APP/Contents/PkgInfo"

cp -R "$ROOT/std" "$APP/Contents/Resources/std"
cp -R "$ROOT/examples" "$APP/Contents/Resources/examples"
mkdir -p "$APP/Contents/Resources/ide"
cp "$ROOT/ide/web/index.html" "$APP/Contents/Resources/ide/index.html"
if [[ -d "$ROOT/runtime" ]]; then
  cp -R "$ROOT/runtime" "$APP/Contents/Resources/runtime"
fi
if [[ -d "$ROOT/tools/avr" ]]; then
  mkdir -p "$APP/Contents/Resources/tools"
  cp -R "$ROOT/tools/avr" "$APP/Contents/Resources/tools/avr"
fi

if [[ -f "$ROOT/ide/macos/AppIcon.icns" ]]; then
  cp "$ROOT/ide/macos/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
fi

find "$APP" \( -name '.DS_Store' -o -name '._*' \) -delete
if command -v codesign >/dev/null; then
  codesign --force --deep --sign - "$APP" >/dev/null
fi

echo "Built $APP"
echo "Open with:  open \"$APP\""
