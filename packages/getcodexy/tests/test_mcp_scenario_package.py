"""Fresh Devtools package checks for the repository-only scenario boundary."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPOSITORY = Path(__file__).resolve().parents[3]
DEVTOOLS = REPOSITORY / "plugins/codexy-devtools"


class DevtoolsScenarioBoundaryTests(unittest.TestCase):
    def _install(self, root: Path) -> Path:
        installed = root / "installed" / "codexy-devtools"
        shutil.copytree(DEVTOOLS, installed)
        return installed

    def test_fresh_package_omits_scenario_tools_and_retains_codegraph_lsp(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            installed = self._install(root)
            for relative in (
                "skills/mcp-test",
                "scripts",
            ):
                self.assertFalse((installed / relative).exists(), relative)
            for relative in (
                ".mcp.json",
                "mcp/codexy_mcp_bootstrap.py",
                "mcp/codexy-mcp-codegraph",
                "mcp/codexy-mcp-lsp",
                "skills/codegraph/SKILL.md",
                "skills/lsp/SKILL.md",
                "lsp/server-catalog.toml",
            ):
                self.assertTrue((installed / relative).is_file(), relative)
            manifest_path = installed / ".codex-plugin/plugin.json"
            manifest_text = manifest_path.read_text(encoding="utf-8")
            manifest = json.loads(manifest_text)
            self.assertNotIn("mcp-testing", manifest["keywords"])
            self.assertNotIn("scenarios", manifest["keywords"])
            self.assertNotIn("$mcp-test", manifest_text)
            self.assertNotIn("scenario-testing", manifest_text)
            self.assertEqual(
                set(json.loads((installed / ".mcp.json").read_text())),
                {"codegraph", "lsp"},
            )
            if os.name != "posix":
                return
            runtime = root / "runtime"
            runtime.mkdir()
            environment = os.environ | {
                "CODEXY_RUNTIME_DIR": str(runtime),
                "CODEXY_RUNTIME_PLATFORM": "darwin-arm64",
            }
            for server in ("codegraph", "lsp"):
                binary = runtime / f"codexy-mcp-{server}-darwin-arm64.bin"
                binary.write_text(f"#!/bin/sh\nprintf '%s\\n' {server}\n")
                binary.chmod(0o755)
                launched = subprocess.run(
                    [str(installed / f"mcp/codexy-mcp-{server}"), "probe"],
                    cwd=installed,
                    env=environment,
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(launched.returncode, 0, launched.stderr)
                self.assertEqual(launched.stdout.strip(), server)


if __name__ == "__main__":
    unittest.main()
