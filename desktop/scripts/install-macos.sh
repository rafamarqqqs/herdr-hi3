#!/usr/bin/env bash
set -euo pipefail
desktop_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
source_app="$desktop_root/src-tauri/target/release/bundle/macos/hi3.app"
target_app="${HOME:?}/Applications/hi3.app"
if [[ ! -d "$source_app" ]]; then
  echo 'Build hi3 first. No app installed.' >&2
  exit 1
fi
if [[ -e "$target_app" ]]; then
  echo 'hi3.app already exists. Refusing to replace it; explicit update workflow required.' >&2
  exit 1
fi
bundle_id=$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$source_app/Contents/Info.plist")
[[ "$bundle_id" == dev.hi3.preview ]] || { echo 'Wrong app identity; stopped.' >&2; exit 1; }
/usr/bin/codesign --verify --deep --strict "$source_app"
mkdir -p "${HOME:?}/Applications"
/usr/bin/ditto "$source_app" "$target_app"
/usr/bin/codesign --verify --deep --strict "$target_app"
echo "Installed separately: $target_app"
echo 'No existing app, terminal binary, configuration or server was replaced.'
