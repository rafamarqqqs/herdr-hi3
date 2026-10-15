# Desktop app development

The Rust client/runtime remains at the repository root. This directory owns
desktop packaging, launcher resources and upstream-update tooling.

- Never bundle user settings, credentials, logs, sessions or local backups.
- Build into ignored `app/build/`; installation and server updates require explicit approval.
- Preserve terminal IDs, processes and negotiated protocol compatibility.
- Keep platform packaging separate from the server and terminal client.
- Follow the upstream code's test and protocol rules. Do not submit this fork's changes upstream without authorization.
- Do not add coauthor or AI-attribution trailers to commits.

hi3 is the new Tauri client in `../desktop/`, starting with a one-terminal macOS
prototype. The scripts in this directory still package the inherited Ghostty app;
do not run them as hi3 build/install commands. See `ROADMAP.md`.

Keep hi3 app identity, build outputs, configuration, state and test sockets
distinct from existing Herdr installations. Never connect tests to an inherited
default server or install over an existing app. Do not bundle personal data.
