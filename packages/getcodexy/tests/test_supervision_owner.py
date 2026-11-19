"""Exercise owner reconciliation through the same private state as dispatch."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HOOK = (
    Path(__file__).resolve().parents[3]
    / "plugins/codexy/hooks/codexy_supervision_owner.py"
)


class OwnerCommandTests(unittest.TestCase):
    def test_bootstrap_accepts_an_explicit_owner_roster(self):
        with tempfile.TemporaryDirectory() as root:
            owner = {
                "request": "r",
                "issue": "i",
                "model": "m",
                "thinking": "t",
                "child": "c",
            }
            result = subprocess.run(
                [
                    sys.executable,
                    "-I",
                    "-B",
                    str(HOOK),
                    "--plugin-data",
                    root,
                    "--parent",
                    "parent",
                    "bootstrap",
                    "--confirmed-complete",
                ],
                input=json.dumps([owner]),
                text=True,
                capture_output=True,
                check=True,
            )
            self.assertEqual(json.loads(result.stdout)["result"], "reconciled")

    def test_adopt_lookup_release_and_resume(self):
        with tempfile.TemporaryDirectory() as root:

            def invoke(*args, check=True):
                result = subprocess.run(
                    [
                        sys.executable,
                        "-I",
                        "-B",
                        str(HOOK),
                        "--plugin-data",
                        root,
                        "--parent",
                        "parent",
                        *args,
                    ],
                    text=True,
                    capture_output=True,
                    check=check,
                )
                return result.returncode, json.loads(result.stdout)

            _, result = invoke(
                "adopt",
                "--request",
                "request",
                "--issue",
                "issue",
                "--model",
                "model",
                "--thinking",
                "medium",
                "--child",
                "child",
            )
            self.assertEqual(result["result"], "adopted")
            _, result = invoke("lookup", "--child", "child")
            self.assertEqual(result["owner"]["issue"], "issue")
            code, _ = invoke(
                "release", "--child", "missing", "--confirmed", check=False
            )
            self.assertNotEqual(code, 0)
            _, result = invoke("release", "--child", "child", "--confirmed")
            self.assertEqual(result["result"], "released")
            _, result = invoke("resume", "--child", "child")
            self.assertEqual(result["result"], "resumed")
            _, result = invoke("resume", "--child", "child")
            self.assertEqual(result["result"], "existing")
