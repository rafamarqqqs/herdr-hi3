import json
from pathlib import Path
import subprocess
import shutil
import tempfile
import unittest
from upstream import prepare


class UpstreamTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.repo = self.base / "upstream"
        self.repo.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Test fixture")
        (self.repo / "file.txt").write_text("base\n")
        (self.repo / "delete.txt").write_text("delete me\n")
        self.git("add", ".")
        self.git("commit", "-m", "base")
        self.sha = self.git("rev-parse", "HEAD")
        self.root = self.base / "app"
        (self.root / "herdr").mkdir(parents=True)
        (self.root / "herdr/file.txt").write_text("custom\n")
        (self.root / "herdr/added.bin").write_bytes(b"\x00\x01local")
        (self.root / "upstream.json").write_text(json.dumps({"repository": str(self.repo), "commit": self.sha}))
        for folder in ("macos", "scripts"):
            (self.root / folder).mkdir()
        for name in ("LICENSE", "README.md", "SOURCE.md", "AGENTS.md", "UPDATING.md"):
            (self.root / name).write_text("fixture\n")

    def git(self, *args):
        return subprocess.check_output(["git", "-c", "core.hooksPath=/dev/null", *args], cwd=self.repo).decode().strip()

    def unchanged(self):
        self.assertEqual((self.root / "herdr/file.txt").read_text(), "custom\n")
        self.assertFalse((self.root / "herdr/upstream.txt").exists())
        self.assertEqual(json.loads((self.root / "upstream.json").read_text())["commit"], self.sha)

    def test_forward_merge_preserves_custom_binary_and_deletion(self):
        (self.repo / "upstream.txt").write_text("new upstream feature\n")
        self.git("add", ".")
        self.git("commit", "-m", "upstream advance")
        target = self.git("rev-parse", "HEAD")
        stage, success = prepare(self.root, target)
        self.assertTrue(success)
        self.assertEqual((stage / "herdr/file.txt").read_text(), "custom\n")
        self.assertEqual((stage / "herdr/added.bin").read_bytes(), b"\x00\x01local")
        self.assertFalse((stage / "herdr/delete.txt").exists())
        self.assertTrue((stage / "herdr/upstream.txt").exists())
        self.assertEqual(json.loads((stage / "upstream.json").read_text())["commit"], target)
        self.unchanged()

    def test_conflict_stops_without_overwriting_original(self):
        (self.repo / "file.txt").write_text("upstream edit\n")
        self.git("add", ".")
        self.git("commit", "-m", "conflict")
        stage, success = prepare(self.root, "main")
        self.assertFalse(success)
        report = json.loads((stage / "update-report.json").read_text())
        self.assertEqual(report["conflicts"], ["file.txt"])
        self.assertFalse(report["validated"])
        self.unchanged()

    def test_invalid_ref_stops_without_overwriting_original(self):
        with self.assertRaises(subprocess.CalledProcessError):
            prepare(self.root, "missing-tag")
        self.unchanged()

    def test_option_is_not_a_ref(self):
        with self.assertRaises(ValueError):
            prepare(self.root, "--help")
        self.unchanged()

    def test_downgrade_is_rejected(self):
        (self.repo / "new.txt").write_text("upstream\n")
        self.git("add", ".")
        self.git("commit", "-m", "new base")
        new_base = self.git("rev-parse", "HEAD")
        (self.root / "upstream.json").write_text(json.dumps({"repository": str(self.repo), "commit": new_base}))
        with self.assertRaises(subprocess.CalledProcessError):
            prepare(self.root, self.sha)
        self.assertEqual((self.root / "herdr/file.txt").read_text(), "custom\n")

    def test_fork_layout_keeps_packaging_out_of_upstream_patch(self):
        fork = self.base / "fork"
        fork.mkdir()
        package = fork / "app"
        self.root.rename(package)
        for path in (package / "herdr").iterdir():
            shutil.copy2(path, fork / path.name)
        (package / "herdr").rename(self.base / "saved-fixture-source")
        stage, success = prepare(package, self.sha)
        self.assertTrue(success)
        self.assertEqual((stage / "herdr/file.txt").read_text(), "custom\n")
        self.assertFalse((stage / "herdr/app").exists())
        self.assertTrue((stage / "macos").is_dir())


if __name__ == "__main__":
    unittest.main()
