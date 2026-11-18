import base64
import importlib.util
from pathlib import Path
import unittest

SCRIPT = Path(__file__).resolve().parents[2] / "scripts/resolve_web_release.py"
spec = importlib.util.spec_from_file_location("resolve_web_release", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseSelectionTest(unittest.TestCase):
    def setUp(self):
        self.sha = "a" * 40
        self.release = {"tag_name": "v2.5.0+606", "draft": False, "prerelease": False}
        self.run = {
            "conclusion": "success", "event": "push", "head_branch": "release",
            "head_repository": {"full_name": "iebb/NekokoLPA2"}, "head_sha": self.sha,
        }
        self.data = {
            "releases/latest": self.release,
            "commits/v2.5.0%2B606": {"sha": self.sha},
            "contents/android/app/build.gradle.kts?ref=" + self.sha: {
                "content": base64.b64encode(
                    b'defaultConfig {\n    versionCode = 606\n    versionName = "2.5.0"\n}\n'
                ).decode(),
            },
        }

    def select(self, kind, event):
        return module.select_release(kind, event, "iebb/NekokoLPA2", self.data.__getitem__)

    def test_published_release_resolves_to_immutable_commit(self):
        result = self.select("release", {"release": self.release})
        self.assertEqual(result, {"tag": "v2.5.0+606", "sha": self.sha, "version": "2.5.0+606"})

    def test_token_created_release_is_covered_by_completed_build(self):
        self.assertEqual(self.select("workflow_run", {"workflow_run": self.run})["sha"], self.sha)

    def test_legacy_pubspec_fallback_still_resolves_older_builds(self):
        self.data["contents/android/app/build.gradle.kts?ref=" + self.sha] = {
            "content": base64.b64encode(
                b"versionCode = flutter.versionCode\nversionName = flutter.versionName\n"
            ).decode(),
        }
        self.data["contents/pubspec.yaml?ref=" + self.sha] = {
            "content": base64.b64encode(b"version: 2.5.0+606\n").decode(),
        }
        self.assertEqual(self.select("workflow_run", {"workflow_run": self.run})["sha"], self.sha)

    def test_delayed_older_release_does_not_roll_back(self):
        old = dict(self.release, tag_name="v2.3.0+589")
        self.assertIsNone(self.select("release", {"release": old}))

    def test_fork_cannot_supply_a_deployment(self):
        self.run["head_repository"]["full_name"] = "someone/NekokoLPA2"
        with self.assertRaises(ValueError):
            self.select("workflow_run", {"workflow_run": self.run})

    def test_failed_build_cannot_deploy(self):
        self.run["conclusion"] = "failure"
        with self.assertRaises(ValueError):
            self.select("workflow_run", {"workflow_run": self.run})

    def test_unpublished_source_at_the_same_version_does_not_deploy(self):
        self.data["commits/v2.5.0%2B606"] = {"sha": "b" * 40}
        self.assertIsNone(self.select("workflow_run", {"workflow_run": self.run}))

    def test_manual_run_defaults_to_latest(self):
        self.assertEqual(self.select("workflow_dispatch", {})["tag"], "v2.5.0+606")

    def test_invalid_manual_tag_is_rejected(self):
        with self.assertRaises(ValueError):
            self.select("workflow_dispatch", {"inputs": {"release_tag": "master"}})

    def test_prerelease_cannot_deploy(self):
        self.assertIsNone(self.select("release", {"release": dict(self.release, prerelease=True)}))


if __name__ == "__main__":
    unittest.main()
