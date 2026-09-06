# Cross-platform desktop direction

Target: an Electron client for Windows, Linux and macOS, backed by the Herdr
runtime. The macOS/Ghostty preview is the current working implementation and
remains supported during exploration.

1. Inventory the existing negotiated JSON/API capabilities and identify missing
   terminal-streaming, resize, input and lifecycle contracts. Do not couple the
   new client to the private TUI wire format without an explicit compatibility plan.
2. Prototype one terminal surface, reconnection and input correctness. Keep
   terminal/process ownership in the server, including detach-with-jobs-running.
3. Port workspace navigation, pane layout/dragging, status and sleep interactions
   into the desktop renderer, with cross-platform keyboard and pointer tests.
4. Add platform packaging, updates, permissions and signing after the core client
   contract is demonstrated on all three target systems.

Security boundaries: sandboxed renderer, context isolation, no renderer Node.js
integration, narrow validated IPC, and no remote content with privileged access.
Keep authentication, credentials and process control out of untrusted UI content.

This roadmap does not introduce Electron dependencies, implement a terminal
renderer, or promise feature parity or a release timeline.
