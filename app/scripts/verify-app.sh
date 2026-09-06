#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
source_root="$root/herdr"
if [[ ! -f "$source_root/Cargo.toml" ]]; then source_root="$root/.."; fi
cd "$source_root"
if [[ -d .git ]] && [[ -n $(git diff --name-only --diff-filter=U) ]]; then
  echo 'Resolve and stage all upstream conflicts before validation.' >&2
  exit 1
fi
# Named guards fail if upstream merges accidentally remove custom regression tests.
for test in \
  tab_click_jitter_does_not_merge_into_the_visible_pane \
  app_sidebar_toggle_has_padding_and_no_status_overlap \
  app_activity_buttons_have_equal_width_gaps_and_outer_margins \
  sleep_is_app_only_and_detaches_without_closing_server_work \
  app_unnamed_pane_handle_repositions_and_ignores_stale_drop_targets \
  api_pane_reposition_preserves_pane_terminal_ids_names_and_root \
  generated_protocol_schema_artifact_is_current
do
  cargo nextest run --locked --no-tests=fail "$test" --status-level fail --final-status-level fail
done
just test-one client::shell::
just test-one frozen
just test-one advertised_client_shell_method_shapes
just ui-hot-path-architecture-test
cd "$root"
bash scripts/build-app.sh
echo 'Automated checks passed. Manual app/old-server compatibility checks are still required; nothing installed.'
