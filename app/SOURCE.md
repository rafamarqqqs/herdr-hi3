# Source provenance

This preview starts from upstream Herdr commit
`3150bd92d6162ba248bc28fd4d40bd0d6238e1af` (package version `0.8.2`).
The upstream Git history is retained.

The current upstream base is recorded in `upstream.json`. The master integration
advances it to `5a64e8da4551990367dd26d77cf3b654dfa27a73`, preserving the app layer
alongside the upstream multi-endpoint client and selection fixes.

Customizations cover the desktop sidebar, hover and touchpad input, tab/pane
dragging, stable-name pane repositioning, and client-only sleep UI/settings.
The packaging layer includes a reconstructed resource-relative C launcher and
Ghostty AppleScript. It is not an official upstream release.

No user credentials, account policies, session data, personal configuration,
application binaries or machine-specific backups are included. Agent workflow
preferences from local development are not part of the exported skill.

The final local client revision passed 174 client tests and 6 UI architecture
checks before source export. Packaging validation compiles the launcher and
checks the bundle signature; it is not a substitute for fresh-machine QA.

The master integration passed `just check` on macOS: 3,128 Rust tests (two
ignored benchmarks), formatting, native and Windows-target lint, maintenance,
UI architecture, integration assets, marketplace and documentation contracts.
The six updater tests and both render-scaling benchmarks also passed. An
isolated PTY test covers app-mode detach/reattach while server work continues.
Interactive Ghostty and older-server compatibility still require manual QA;
these checks do not install, restart or promote the candidate app.
