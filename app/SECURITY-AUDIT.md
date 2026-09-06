# Source disclosure audit

Scope: the published desktop customization commit `2da8886f` and all 1,528
commits reachable in the fetched repository at audit time (2026-09-06).

Gitleaks 8.30.1 ran locally with full redaction and inline allow-comments ignored.
The desktop customization commit produced no findings. The complete history
produced five generic-api-key alerts, all in the upstream vendored Ghostty import
`41dd733ca7123c9ac4bfc6c781697a660e95b052`:

- Four notarization workflow statements reference an environment variable and
  the filename `notarization_key.p8`; they do not contain a key value.
- One Zig keybinding test defines a chained close-surface shortcut, not an API
  credential.

These alerts were inspected in their original commit context and classified as
false positives. No vendor history was rewritten and no blanket scanner allowlist
was added. Additional checks found no development-machine paths, unrelated
project names or private account-policy files in the published customization.

No actual credentials were identified. Pattern-based scans cannot prove the
absence of every possible sensitive value. Repeat the scan before releases and
investigate new findings; never publish unredacted scan reports.
