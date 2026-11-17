import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from scripts.release_metadata import build_metadata, stamp, validate_pr_update


class ReleaseMetadataTests(unittest.TestCase):
    def setUp(self):
        self.repo = Path(tempfile.mkdtemp())
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Scalia tests")
        self.git("config", "user.email", "tests@example.invalid")
        (self.repo / "VERSION").write_text("Alpha 0.1\n", encoding="utf-8")
        self.commit("Start version history")

    def tearDown(self):
        import shutil

        shutil.rmtree(self.repo)

    def git(self, *args):
        return subprocess.run(
            ["git", *args], cwd=self.repo, check=True, capture_output=True, text=True
        ).stdout.strip()

    def commit(self, message):
        (self.repo / "change.txt").write_text(message.splitlines()[0], encoding="utf-8")
        self.git("add", ".")
        self.git("commit", "-m", message)

    def merge_update(self, branch, title, area, number):
        self.git("checkout", "-b", branch)
        self.commit(f"{title}\n\nA player-facing summary.\n\nUpdate: {area}\n\nCo-authored-by: Copilot")
        self.git("checkout", "main")
        self.git(
            "merge",
            "--no-ff",
            branch,
            "-m",
            f"Merge pull request #{number} from example/{branch}",
            "-m",
            title,
        )

    def test_merge_commits_get_one_area_version_and_pr_reference(self):
        self.merge_update("feature-one", "Add mission plans", "AI", 42)
        self.merge_update("feature-two", "Add squad briefings", "AI", 43)

        metadata = build_metadata(self.repo)

        self.assertEqual(metadata["n"], "AI update V 002 · PR #43")
        self.assertEqual(metadata["title"], "PR #43")
        self.assertEqual(metadata["version"], "Alpha 0.1.2")
        self.assertEqual(metadata["changelog"][0]["n"], "AI update V 002 · PR #43")
        self.assertEqual(metadata["changelog"][1]["n"], "AI update V 001 · PR #42")
        self.assertEqual(metadata["changelog"][0]["text"], "Add squad briefings")
        self.assertEqual(metadata["changelog"][0]["body"], "A player-facing summary.")

    def test_stamp_writes_the_specific_version_and_pull_to_game_metadata(self):
        self.merge_update("feature", "Add mission plans", "AI", 42)
        web = self.repo / "web"
        web.mkdir()
        (web / "game.js").write_text(
            "const UPDATE = '__UPDATE__'; const VERSION = '__VERSION__';",
            encoding="utf-8",
        )

        stamp(self.repo)

        self.assertIn("AI update V 001 · PR #42", (web / "game.js").read_text(encoding="utf-8"))
        version = json.loads((web / "version.json").read_text(encoding="utf-8"))
        changelog = json.loads((web / "changelog.json").read_text(encoding="utf-8"))
        self.assertEqual(version["n"], "AI update V 001 · PR #42")
        self.assertEqual(version["version"], "Alpha 0.1.1")
        self.assertEqual(changelog[0]["n"], "AI update V 001 · PR #42")

    def test_pr_requires_matching_specific_area_in_body_and_head_commit(self):
        area = validate_pr_update(
            "Summary\n\nUpdate: AI", "Implement mission planning\n\nUpdate: AI\n\nCo-authored-by: Copilot"
        )
        self.assertEqual(area, "AI")
        with self.assertRaisesRegex(ValueError, "PR description says"):
            validate_pr_update("Update: AI", "Implement mission planning\n\nUpdate: Animation")
        with self.assertRaisesRegex(ValueError, "specific update area"):
            validate_pr_update("Update: Build", "Add code\n\nUpdate: Build")

    def test_latest_untagged_commit_does_not_fall_back_to_build(self):
        with self.assertRaisesRegex(ValueError, "needs an `Update: <Area>`"):
            build_metadata(self.repo)


if __name__ == "__main__":
    unittest.main()
