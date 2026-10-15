#!/bin/sh
# Package a fresh path without replacing the bundle which may currently be open.
set -eu
cd "$(dirname "$0")/.."
base=src-tauri/target/release/bundle/macos/hi3.app
if ! test -d "$base"; then
  base=src-tauri/target/release/bundle/macos/.app-backups.noindex/hi3.app
fi
test -d "$base"
bun run tauri build --no-bundle
mkdir -p build
candidate_dir=$(mktemp -d "$PWD/build/panes-candidate-XXXXXX")
ditto "$base" "$candidate_dir/hi3.app"
cp src-tauri/target/release/hi3 "$candidate_dir/hi3.app/Contents/MacOS/hi3"
codesign --force --sign - "$candidate_dir/hi3.app/Contents/MacOS/hi3"
codesign --force --sign - "$candidate_dir/hi3.app"
codesign --verify --deep --strict "$candidate_dir/hi3.app"
printf '%s\n' "$candidate_dir/hi3.app"
