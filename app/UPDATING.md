# Updating the customized desktop build

`upstream.json` pins the upstream base. From the repository root:

```sh
python3 app/scripts/upstream.py <upstream-tag-or-commit>
```

The tool generates a full local customization patch against that base and applies
it in three-way mode onto the selected revision. The candidate is created under
`app/build/upstream-…/`, with Rust source in `herdr/` and app packaging beside it.
`app/` is excluded from the upstream patch and copied independently. The original
source, pinned base, installed app and running server are not modified.

On conflicts, preparation fails and retains the candidate for deliberate review.
Inspect its `herdr/` Git status, resolve conflicts and stage resolutions. Review
the entire custom patch; an application error can also leave an incomplete patch
without conflict markers. Never approve based only on the absence of markers.

From the candidate root run `bash scripts/verify-app.sh`. This checks required
custom tests, client regressions, frozen protocols, schema generation and UI
architecture before building a separate app. It fails if required tests disappear.
No installation occurs. Test touchpad/dragging, names, sidebar, menus, sleep,
optional plugins and old-server capability fallback manually before adopting.

Adoption is manual and reviewable: commit a recovery point, review the candidate,
replace the upstream source excluding its private `.git` and build artifacts,
preserve this repository's `app/` directory, and update `app/upstream.json` in the
same source change. Keep the previous source tag and release app. Never stop or
downgrade an active server as an automatic installation step.

The tool accepts `--repository /path/to/upstream-clone` for offline testing.
Compatibility is checked per revision, not guaranteed for arbitrary future
upstream releases. Do not use upstream's binary self-updater for this fork.
