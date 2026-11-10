"""Exercise rollback export through a copied plugin and authenticated bridge."""

import json
import subprocess
import sys
import unittest

import test_resumable_context_capsule as capsule_tests


class ReplayExportLauncherTest(unittest.TestCase):
    def test_copied_plugin_verifies_bridge_and_preserves_source(self):
        fixture = capsule_tests.ResumableContextCapsuleTests(methodName="runTest")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        manifest = fixture._install_runtime()
        replay = fixture.plugins.parent / "replay.json"
        capsule = fixture._capsule("compaction", replay)
        self.assertEqual(fixture._run(capsule).returncode, 0)
        before = replay.read_bytes()
        authority = capsule.parent / "authority" / capsule.name
        output = fixture.plugins.parent / "previous-replay.json"
        script = fixture.plugin / "skills/self-improvement/scripts/export_previous_replay.py"
        command = [sys.executable, str(script), "--replay", str(replay), "--authority",
                   str(authority), "--output", str(output)]
        result = subprocess.run(command, text=True, capture_output=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["schema"], "codexy.previous-replay-export.v1")
        self.assertEqual(replay.read_bytes(), before)
        self.assertEqual(len(json.loads(output.read_text())), 1)

        document = json.loads(manifest.read_text())
        platform_id = f"{capsule_tests.platform.system().lower()}-{capsule_tests.MACHINES[capsule_tests.platform.machine()]}"
        document["platforms"][platform_id]["sha256"] = "0" * 64
        manifest.write_text(json.dumps(document))
        rejected = output.with_name("rejected.json")
        command[-1] = str(rejected)
        self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)
        self.assertFalse(rejected.exists())


if __name__ == "__main__":
    unittest.main()
