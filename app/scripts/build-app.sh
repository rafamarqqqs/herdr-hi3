#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
source_root="$root/herdr"
if [[ ! -f "$source_root/Cargo.toml" ]]; then source_root="$root/.."; fi
if [[ $(uname -s) != Darwin ]]; then
  echo 'Building the app requires macOS and Xcode Command Line Tools.' >&2
  exit 1
fi
if [[ $# == 0 ]]; then
  (cd "$source_root" && cargo build --release --locked)
  binary="${CARGO_TARGET_DIR:-$source_root/target}/release/herdr"
  if [[ $binary != /* ]]; then binary="$source_root/$binary"; fi
elif [[ $# == 2 && $1 == --binary && $2 == /* && -f $2 ]]; then
  binary=$2
else
  echo 'Usage: bash scripts/build-app.sh [--binary /absolute/path/to/herdr]' >&2
  exit 2
fi
# Never overwrite the imported snapshot or an installed/running app.
mkdir -p "$root/build"
stage=$(mktemp -d "$root/build/package-XXXXXX")
app="$stage/Herdr.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$root/macos/Info.plist" "$app/Contents/Info.plist"
cp "$root/macos/Herdr.icns" "$app/Contents/Resources/Herdr.icns"
cp "$root/macos/launch-ghostty.applescript" "$app/Contents/Resources/launch-ghostty.scpt"
cp "$binary" "$app/Contents/Resources/herdr"
cp "$root/LICENSE" "$app/Contents/Resources/LICENSE"
cp "$root/SOURCE.md" "$app/Contents/Resources/SOURCE.md"
cp "$root/upstream.json" "$app/Contents/Resources/upstream.json"
xcrun clang -Wall -Wextra -Werror -O2 -mmacosx-version-min=13.0 \
  "$root/macos/HerdrLauncher.c" -o "$app/Contents/MacOS/HerdrLauncher"
codesign --force --sign "${SIGN_IDENTITY:--}" "$app/Contents/Resources/herdr"
codesign --force --sign "${SIGN_IDENTITY:--}" "$app/Contents/MacOS/HerdrLauncher"
codesign --force --sign "${SIGN_IDENTITY:--}" "$app"
codesign --verify --deep --strict "$app"
plutil -lint "$app/Contents/Info.plist"
echo "Built: $app"
