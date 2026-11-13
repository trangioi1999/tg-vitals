#!/usr/bin/env bash
# Build a universal (Apple silicon + Intel) tg-vitals.app, ad-hoc signed.
set -euo pipefail

cd "$(dirname "$0")/../app"

npm ci
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run tauri build -- --target universal-apple-darwin --bundles app

APP="src-tauri/target/universal-apple-darwin/release/bundle/macos/tg-vitals.app"
echo "✅ app/$APP"
echo "   Chạy:  open app/$APP"
echo "   Cài:   cp -R app/$APP /Applications/"
