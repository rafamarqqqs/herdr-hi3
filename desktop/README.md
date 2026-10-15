# hi3 — macOS navigation candidate

Independent Tauri app (`dev.hi3.preview`), not a replacement for the Ghostty app
or the official `herdr` command. This candidate adds compact server-backed
Spaces, tabs and Agents navigation, new Space/tab actions, top-aligned sidebar
collapse, subtle hover and a working menu with window-local font settings.
The current Ghostty app is the visual/behavior reference; this is not full parity.
Multiple panes in the selected tab are rendered simultaneously using server layout
geometry, with independent terminal controllers and explicit 50/50 right/down
split buttons. Drag, CPU/RAM, sleep and an explorer are not implemented.
Zoom and simultaneous duplicate views of the same terminal show an explicit
limitation rather than competing for control. Up to 32 views can be attached.
There is no external-sidebar plugin dependency.

## Visual agent states

Open **menu → agent states demo** for six simulated agents spanning working,
idle, blocked, done and unknown, plus a plain shell excluded from Agents.
The initial tab contains two panes. Hover and click real rows to compare normal,
hovered and selected styling; collapse also shows the status icons.
The yellow DEMO label and terminal text identify simulation. Typing executes
nothing, tab creation/splits are disabled, and no fake agents are registered with
the server. **menu → return to test server** restores real test navigation.
Switching releases GUI controllers only; server processes remain running.

`new` creates and selects a Space directly in the selected pane's current
directory, with Herdr's default name. It does not prompt for a path or use another
client's focus. In the demo this creates an in-memory Space only. Menu is a popup
anchored to its button; click again, click outside or press Escape to dismiss.
Arrow keys select menu items and Enter activates them. This is menu navigation,
independent of pane shortcuts.

`MENU-PARITY.md` records the actual Herdr menu and settings inventory.

## Pane shortcuts (i3-inspired preview)

Use Command+Option as Mod. Mod+arrows (or J/K/L/;) focuses left/down/up/right;
adding Shift repositions the selected pane beside its neighbor in the same tab.
Mod+H/V selects the next split orientation (right/down), then Mod+Enter creates
the pane. Mod+F toggles client-local zoom. Mod+R enters resize mode: arrows resize
by 5%; Escape or Enter exits. Dialogs and IME composition are excluded. There is
no close/kill shortcut, floating mode or full i3 parent-container navigation.
Demo supports focus and zoom; layout mutations require the isolated test server.
Menu → keybinds shows the mapping; build label is `i3-preview-01`.

The bundled fork runtime owns the shell. The GUI uses its public JSON API and
`terminal session control` stream, not a copied private protocol implementation.
Closing the GUI only closes the controller; the isolated server and work remain.
Rendering is ANSI screen diffs, not a raw PTY stream; graphics are not supported.

## Isolation

Data lives in macOS Application Support under `dev.hi3.preview`. Config, state,
logs, working directory and sockets are dedicated to hi3. No default-server
fallback, takeover, automatic update or connection to existing user sessions.
One GUI owns a lock; a second instance is rejected instead of stealing control.
The prototype uses zsh with an empty app-owned ZDOTDIR. It does not import the
user's shell configuration or inherited credential variables. This is process
and configuration isolation, **not a filesystem sandbox**: commands you type
still have your normal user permissions.

The IPC exposes connect/input/resize/scroll/read-frame, a coherent public session
snapshot, and explicit Space/tab creation. It does not expose generic RPC or
generic move/merge/close operations. A narrow pane action supports same-tab
reposition and directional resize. Selection is client-local and preserves pane context
even when multiple panes reference the same terminal. New tabs derive their cwd
from the selected pane, not the server's independently focused pane.
Frontend assets
are local, with CSP and no shell/filesystem plugin. OSC clipboard writes and
hyperlinks are disabled; copying selected text remains an explicit user action.
The output channel is bounded to four frames, each at most 4 MiB of JSON, and
the frontend waits for xterm to consume a frame before requesting another.
Pipe writes time out after 500 ms instead of indefinitely blocking the window.
Non-reused controller leases and serialized switching prevent stale input from
reaching another terminal. Each controller has its own lock. Hidden views stop
accepting keyboard, paste and wheel input before they detach. Navigation buttons retain DOM identity during refresh
so polling does not replace the target beneath a touchpad click.
Paste is explicitly enveloped for Herdr's live-mode-aware paste handling rather
than relying on xterm to infer child modes from screen diffs. Escape/NUL-containing
clipboard text is rejected. Full application-cursor/mouse mode parity with TUIs
is not yet validated; this remains a shell-first prototype.

## Build

Requires macOS, Xcode tools, Rust, Bun and the runtime's Zig prerequisites.
Build the fork runtime with its own target (never the installed binary):

```sh
cd ..
cargo build --release --locked
mkdir -p desktop/src-tauri/runtime
cp target/release/herdr desktop/src-tauri/runtime/herdr
cd desktop
bun install --frozen-lockfile
bun run tauri build --bundles app
```

Output: `src-tauri/target/release/bundle/macos/hi3.app`. Nothing installs itself.
If that bundle is currently open, use `sh scripts/package-candidate-macos.sh`
instead: it builds without bundling and packages into a fresh `build/` directory,
preserving the existing bundle. It requires the previous bundle as a packaging
template; it never launches, installs or stops an app.
For an explicitly approved installed-preview update, stop only the hi3 GUI, then
run `bash scripts/update-macos.sh /absolute/candidate/hi3.app /absolute/Applications/hi3.app`.
It validates bundle identities/signatures and preserves the old app in
`Applications/.hi3-backups/`. It never stops the persistent server.
During this initial bring-up, the runtime resource was copied from the already
validated fork candidate at source base `f82ddb49`, not from an installed app.
The source core is unchanged from that candidate. Both Cargo and Bun locks must
be retained; runtime binaries and build artifacts are ignored.

## Validation

```sh
bun run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
HI3_TEST_RUNTIME_BIN="$PWD/src-tauri/runtime/herdr" cargo test \
  --manifest-path src-tauri/Cargo.toml --locked isolated_runtime_lifecycle -- --ignored
```

The lifecycle test creates and stops only its own temporary server. GUI close
never stops the persistent hi3 server. Do not use test cleanup against a user's
session. Old-server compatibility and Windows/Linux support are not claimed.

Navigation tests cover non-mutating projection, real idle agents versus shells,
deduplication, remembered selection, shared terminal views and explicit names.
The real-server test verifies command output, Unicode, paste, resize, detach,
switching A→B→A with the same shell PID, and invalid-target rejection without
creating a new Space. `just check` includes JS tests, Rust tests, format and clippy.
Multi-pane tests additionally cover nested/nonzero-origin geometry, invalid or
incomplete layouts, late attachment cleanup, independent input routing, stale
leases, pool capacity and independent failures. The real-server test verifies
simultaneous controllers with separate input and terminal sizes, and a surviving
shell PID after detaching its sibling view.

Visual/native interaction QA remains required before installation. A browser-only
fixture is available with `bun scripts/preview-ui.js` after building frontend assets;
it uses simulated data and cannot execute commands or control real sessions.
The automation environment had no browser available during this candidate's checks.
