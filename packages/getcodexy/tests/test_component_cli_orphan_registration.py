"""An isolated doctor invocation must fail visibly on orphaned role files."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class ComponentCliOrphanRegistrationTests(unittest.TestCase):
    def test_doctor_exit_code_reports_orphaned_roles_without_removing_them(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            home = root / "codex-home"
            core_role = home / "agents/codexy/codexy-sentinel.toml"
            github_role = home / "agents/codexy-github/codexy-weaver.toml"
            core_role.parent.mkdir(parents=True)
            github_role.parent.mkdir(parents=True)
            core_role.write_bytes(b"# CODEXY MANAGED AGENT\nname = 'sentinel'\n")
            github_role.write_bytes(b"# Managed by Codexy GitHub.\nname = 'weaver'\n")
            before = (core_role.read_bytes(), github_role.read_bytes())
            codex = root / "codex"
            codex.write_text(
                "#!/usr/bin/env python3\n"
                "import json, sys\n"
                "if sys.argv[1:] == ['plugin', 'list', '--json']:\n"
                "    value = {'installed': []}\n"
                "elif sys.argv[1:] == ['plugin', 'marketplace', 'list', '--json']:\n"
                "    value = {'marketplaces': []}\n"
                "else:\n"
                "    value = {}\n"
                "print(json.dumps(value))\n",
                encoding="utf-8",
            )
            codex.chmod(0o700)
            environment = os.environ.copy()
            environment["PYTHONNOUSERSITE"] = "1"
            environment["PYTHONPATH"] = str(Path(__file__).resolve().parents[1] / "src")

            result = subprocess.run(
                [
                    sys.executable,
                    "-B",
                    "-m",
                    "codexy_runtime_tools.component_cli",
                    "--codex",
                    str(codex),
                    "--codex-home",
                    str(home),
                    "doctor",
                    "--json",
                ],
                capture_output=True,
                check=False,
                env=environment,
                text=True,
            )

            receipt = json.loads(result.stdout)
            health = {
                entry["component"]: entry for entry in receipt["component_health"]
            }
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertEqual(receipt["inventory"]["state"], "absent")
            self.assertEqual(
                {
                    name
                    for name, entry in health.items()
                    if entry["reason_code"] == "orphaned-managed-registration"
                },
                {"core", "github"},
            )
            self.assertEqual((core_role.read_bytes(), github_role.read_bytes()), before)
