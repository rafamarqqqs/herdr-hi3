# Herdr menu reference

Source: `src/client/shell/global_menu.rs`, `settings.rs`, `state.rs` in this fork.
This inventory is not a promise that every action already exists in hi3.

| Herdr menu | Behavior | hi3 status |
| --- | --- | --- |
| settings | Open settings; badge when integrations have updates | Font setting only |
| keybinds | Open shortcut help | Deferred until the i3 interaction design |
| reload config | Reload configuration | Not implemented |
| what's new / update ready | Conditional release notes/update information | No updater connection; custom app must not overwrite itself with upstream binaries |
| detach | Leave the client, preserve runtime work | Window close preserves work; menu action pending |

Settings sections: theme, indicators, sound, toasts, integrations, sleep.
Sleep is app-specific (hidden in the ordinary zero-top-padding terminal client).
Menu supports toggling, highlighted selection, keyboard movement and activation.
hi3 now reproduces the launcher-anchored popup, second-click/outside-click
dismissal and arrow/Enter/Escape interaction. The action inventory above remains
partial. The `new` toolbar action directly creates a Space from the selected
pane's foreground cwd (fallback: pane cwd), with the runtime's default label.

The hi3-only visual demo uses an in-memory runtime adapter and never registers
fake agents or starts processes. Working, idle, blocked, done and unknown are
agent lifecycle states. Hover, selected and keyboard focus are independent UI
states, not additional agent statuses. Use real pointer interaction to inspect
hover; only one row is selected, and selected styling wins over hover.

Next: settle menu scope and layout/focus behavior, then design i3 keybindings.
Do not install keybindings globally or intercept terminal shortcuts prematurely.
