#!/usr/bin/env bash
# Explicit, recoverable update of the isolated preview only. Stop its GUI first.
set -euo pipefail
source_app="${1:?candidate hi3.app required}"
target_app="${2:?installed hi3.app required}"
[[ "$source_app" = /* && "$target_app" = /* && "$target_app" == */Applications/hi3.app && "$source_app" != "$target_app" ]]
for app in "$source_app" "$target_app"; do
  [[ -d "$app" && ! -L "$app" ]]
  [[ "$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$app/Contents/Info.plist")" == dev.hi3.preview ]]
  /usr/bin/codesign --verify --deep --strict "$app"
done
parent=$(dirname "$target_app")
[[ ! -L "$parent" ]]
stage=$(mktemp -d "$parent/.hi3-update-XXXXXX")
/usr/bin/ditto "$source_app" "$stage/hi3.app"
/usr/bin/codesign --verify --deep --strict "$stage/hi3.app"
backup_root="$parent/.hi3-backups"
[[ ! -L "$backup_root" ]]
mkdir -p "$backup_root"
backup=$(mktemp -d "$backup_root/stable-XXXXXX")
mv "$target_app" "$backup/hi3.app"
if ! mv "$stage/hi3.app" "$target_app"; then
  mv "$backup/hi3.app" "$target_app"
  exit 1
fi
rmdir "$stage"
printf 'Installed: %s\nBackup preserved: %s\n' "$target_app" "$backup/hi3.app"
