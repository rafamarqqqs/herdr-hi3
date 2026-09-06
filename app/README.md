# Herdr desktop preview

An independent desktop customization of Herdr. The current implementation uses
Ghostty on macOS. Electron support for Windows, Linux and macOS is planned, not
implemented. No cross-platform desktop release is claimed here.

## Layout

- Repository root: customized Rust client/server and regression tests.
- `app/macos/`: C launcher, AppleScript, icon and bundle metadata.
- `app/scripts/`: build, validation and isolated upstream-update tooling.
- `app/upstream.json`: exact upstream base for preserving customizations.
- `app/ROADMAP.md`: future Electron direction.

## Build and validate

Requires macOS, Xcode Command Line Tools, the Rust toolchain pinned at the repo
root, the upstream Zig/build dependencies, and Ghostty installed separately.

```sh
bash app/scripts/build-app.sh
bash app/scripts/verify-app.sh
python3 -m unittest discover -s app/scripts -p test_upstream.py
```

Builds go to a unique directory under `app/build/`. Nothing is installed,
launched or stopped automatically. Signing is ad-hoc by default; `SIGN_IDENTITY`
selects a local signing identity. Public distribution still requires packaging,
signing/notarization decisions and fresh-machine validation.

For packaging an existing custom binary, use
`bash app/scripts/build-app.sh --binary /absolute/path/to/herdr`. This does not
prove that an arbitrary supplied binary matches the checked-out source.

The app creates a Ghostty window and attaches the bundled Herdr client. It does
not change global Ghostty settings or install a CLI into PATH. The current
launcher uses the default Herdr configuration/server. Optional Explorer actions
require a compatible sidebar plugin. Missing server methods must remain local
action errors, not reasons to stop existing sessions.

See [UPDATING.md](UPDATING.md) for upstream maintenance. Do not use the official
binary updater to replace this customized build. Downloads/Homebrew releases are
not configured by this source import.

## Licensing and scope

Upstream licenses and notices are retained. Ghostty is an external application,
not a bundled binary. Review inherited icon/branding and dependency notices
before distributing release artifacts. The bundle's current `dev.herdr.local`
identifier is development metadata, not a final public application identity.
