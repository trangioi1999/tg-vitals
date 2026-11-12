#!/usr/bin/env bash
# Build a release binary and wrap it into build/tg-vitals.app (ad-hoc signed).
set -euo pipefail

cd "$(dirname "$0")/.."

APP="build/tg-vitals.app"

swift build -c release --arch arm64
BIN="$(swift build -c release --arch arm64 --show-bin-path)/TGVitals"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/TGVitals"
cp Resources/Info.plist "$APP/Contents/Info.plist"
[ -f Resources/AppIcon.icns ] && cp Resources/AppIcon.icns "$APP/Contents/Resources/"

codesign --force --deep --sign - "$APP"

echo "✅ $APP"
echo "   Chạy:  open $APP"
echo "   Cài:   cp -R $APP /Applications/"
