# Cross-platform desktop direction

Target: hi3, a Tauri client backed by the Herdr runtime. The first MVP targets
macOS; Windows and Linux remain later goals. The existing macOS/Ghostty app and
official terminal installation are preserved independently, not migrated in place.

1. Inventory the existing negotiated JSON/API capabilities and identify missing
   terminal-streaming, resize, input and lifecycle contracts. Do not couple the
   new client to the private TUI wire format without an explicit compatibility plan.
2. Prototype one terminal surface, reconnection and input correctness. Keep
   terminal/process ownership in the server, including detach-with-jobs-running.
3. Port workspace navigation, pane layout/dragging, status and sleep interactions
   into the desktop renderer, with cross-platform keyboard and pointer tests.
4. Add platform packaging, updates, permissions and signing after the core client
   contract is demonstrated on all three target systems.

Security requirements: narrowly scoped Tauri capabilities, validated IPC,
untrusted terminal content, and no remote content with privileged access.
Keep authentication, credentials and process control out of untrusted UI content.

Before running a prototype, establish distinct app identity, build outputs,
configuration, state and test sockets. Never reuse the existing default server
or silently fall back to it. The Herdr server must survive GUI detachment.

The first prototype now lives in `../desktop/`: Tauri, vanilla JavaScript and
xterm, using the public terminal-session controller and an isolated Herdr server.
It implements one terminal, bounded input/output, semantic paste, resize and
reconnect. Unit tests and an isolated real-server lifecycle test cover the first
slice; native visual/clipboard/IME QA remains necessary. This is not feature parity
with the Ghostty app. Existing Ghostty packaging scripts are retained but are not
hi3 build instructions. See `../desktop/README.md` for build and isolation details.
