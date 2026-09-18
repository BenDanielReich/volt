#!/bin/bash
# Download Arduino AVR gcc + avrdude into tools/avr (cached).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OS="$(uname -s)"
CACHE="$ROOT/tools/cache"
DEST="$ROOT/tools/avr"
mkdir -p "$CACHE"

if [[ -x "$DEST/avr/bin/avr-gcc" ]] || find "$DEST" -name 'avr-gcc' -type f 2>/dev/null | grep -q .; then
  echo "AVR tools already in $DEST"
  exit 0
fi

if [[ "$OS" == Darwin ]]; then
  GCC_URL="https://downloads.arduino.cc/tools/avr-gcc-7.3.0-atmel3.6.1-arduino7-x86_64-apple-darwin14.tar.bz2"
  DUDE_URL="https://downloads.arduino.cc/tools/avrdude-6.3.0-arduino17-x86_64-apple-darwin12.tar.bz2"
else
  GCC_URL="https://downloads.arduino.cc/tools/avr-gcc-7.3.0-atmel3.6.1-arduino7-x86_64-pc-linux-gnu.tar.bz2"
  DUDE_URL="https://downloads.arduino.cc/tools/avrdude-6.3.0-arduino17-x86_64-pc-linux-gnu.tar.bz2"
fi

fetch() {
  local url="$1"
  local out="$CACHE/$(basename "$url")"
  if [[ ! -f "$out" ]]; then
    echo "Downloading $(basename "$url")…" >&2
    curl -L --fail --retry 2 -o "$out" "$url"
  fi
  printf '%s\n' "$out"
}

GCC_TAR="$(fetch "$GCC_URL")"
DUDE_TAR="$(fetch "$DUDE_URL")"
mkdir -p "$DEST"
echo "Unpacking AVR gcc…"
tar -xjf "$GCC_TAR" -C "$DEST"
echo "Unpacking avrdude…"
tar -xjf "$DUDE_TAR" -C "$DEST"
cat > "$DEST/README.txt" <<'EOF'
Arduino AVR toolchain bundled with Volt (avr-gcc + avrdude).
GCC and avrdude are GPL — source is on downloads.arduino.cc / Arduino GitHub.
EOF
echo "Installed AVR tools in $DEST"
