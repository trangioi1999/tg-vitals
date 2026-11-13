#!/usr/bin/env bash
# Build the Tauri app into app/src-tauri/target/release/bundle/macos/tg-vitals.app (ad-hoc signed).
set -euo pipefail

cd "$(dirname "$0")/../app"

npm ci
npm run tauri build -- --bundles app

APP="src-tauri/target/release/bundle/macos/tg-vitals.app"
echo "✅ app/$APP"
echo "   Chạy:  open app/$APP"
echo "   Cài:   cp -R app/$APP /Applications/"
