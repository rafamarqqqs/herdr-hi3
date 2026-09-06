#!/usr/bin/env python3
"""Prepare an isolated three-way upstream update. Never install or stop Herdr."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def git(*args, cwd=None, env=None, check=True):
    return subprocess.run(
        ["git", "-c", "core.hooksPath=/dev/null", *map(str, args)],
        cwd=cwd, env=env, check=check, stdout=subprocess.PIPE,
    )


def prepare(root, ref, repository=None):
    root = root.resolve()
    source = root / "herdr" if (root / "herdr").is_dir() else root.parent
    lock = json.loads((root / "upstream.json").read_text())
    if ref.startswith("-") or not ref:
        raise ValueError("Use an explicit upstream tag or commit, not a Git option")
    build = root / "build"
    build.mkdir(exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix="upstream-", dir=build))
    print(f"Preparing candidate in {stage}", flush=True)
    objects = stage / "upstream.git"
    git("clone", "--bare", "--no-hardlinks", "--", repository or lock["repository"], objects)
    base = git("--git-dir", objects, "rev-parse", "--verify", f'{lock["commit"]}^{{commit}}').stdout.decode().strip()
    target = git("--git-dir", objects, "rev-parse", "--verify", f"{ref}^{{commit}}").stdout.decode().strip()
    # Refuse downgrades/unrelated trees: they need an explicit migration design.
    git("--git-dir", objects, "merge-base", "--is-ancestor", base, target)

    # A private index computes the complete local delta against the exact base.
    # It never touches the user's index or commits anything.
    env = dict(os.environ, GIT_INDEX_FILE=str(stage / "local.index"))
    git("--git-dir", objects, "read-tree", base, env=env)
    paths = ["."] if source == root / "herdr" else [".", ":(exclude)app"]
    git("--git-dir", objects, "--work-tree", source, "add", "-A", "--", *paths,
        cwd=source, env=env)
    delta = git("--git-dir", objects, "diff", "--cached", "--binary", "--full-index", base, env=env).stdout
    patch = stage / "customizations.patch"
    patch.write_bytes(delta)

    candidate = stage / "herdr"
    git("clone", "--no-hardlinks", "--", objects, candidate)
    git("checkout", "--detach", target, cwd=candidate)
    result = git("apply", "--3way", "--index", patch, cwd=candidate, check=False) if delta else None
    conflicts = git("diff", "--name-only", "--diff-filter=U", cwd=candidate).stdout.decode().splitlines()
    # App packaging remains separate from the upstream tree.
    for directory in ("macos", "scripts"):
        shutil.copytree(root / directory, stage / directory, ignore=shutil.ignore_patterns("__pycache__"))
    for name in ("LICENSE", "README.md", "SOURCE.md", "AGENTS.md", "UPDATING.md"):
        shutil.copy2(root / name, stage / name)
    if (root / "ROADMAP.md").exists():
        shutil.copy2(root / "ROADMAP.md", stage / "ROADMAP.md")
    (stage / "upstream.json").write_text(json.dumps({"repository": lock["repository"], "commit": target}, indent=2) + "\n")
    report = {"base": base, "target": target, "conflicts": conflicts,
              "patch_applied": result is None or result.returncode == 0,
              "validated": False}
    (stage / "update-report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"Candidate: {stage}\nBase: {base}\nTarget: {target}")
    if not report["patch_applied"]:
        print("STOP: merge needs review. See customizations.patch and git status in candidate/herdr.")
        return stage, False
    print("Merged, NOT validated. Review the diff, then run bash scripts/verify-app.sh inside the candidate.")
    print("The source tree, upstream lock, installed app and live server were not changed.")
    return stage, True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ref", help="Explicit upstream tag or commit (no automatic latest)")
    parser.add_argument("--repository", help="Local upstream clone or alternative fetch URL (offline testing)")
    args = parser.parse_args()
    try:
        _, success = prepare(Path(__file__).resolve().parent.parent, args.ref, args.repository)
        return 0 if success else 1
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"Update stopped safely: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
