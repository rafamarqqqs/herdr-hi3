# Desktop app development

The Rust client/runtime remains at the repository root. This directory owns
desktop packaging, launcher resources and upstream-update tooling.

- Never bundle user settings, credentials, logs, sessions or local backups.
- Build into ignored `app/build/`; installation and server updates require explicit approval.
- Preserve terminal IDs, processes and negotiated protocol compatibility.
- Keep platform packaging separate from the server and terminal client.
- Follow the upstream code's test and protocol rules. Do not submit this fork's changes upstream without authorization.
- Do not add coauthor or AI-attribution trailers to commits.

Electron is a future client target, not an implemented dependency or supported
runtime in this preview. See `ROADMAP.md` before proposing cross-platform work.
