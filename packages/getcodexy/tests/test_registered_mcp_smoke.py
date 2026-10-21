"""Exercise the registration smoke script's core absence and strict failure paths."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPOSITORY = Path(__file__).resolve().parents[3]
SUPPORTED_PLATFORMS = [
    "linux-x86_64",
    "linux-arm64",
    "darwin-x86_64",
    "darwin-arm64",
    "windows-x86_64",
    "windows-arm64",
]


@unittest.skipUnless(
    shutil.which("pwsh"), "PowerShell is required for smoke-script tests"
)
class RegisteredMcpSmokeTests(unittest.TestCase):
    def _run_smoke(
        self,
        core_manifest: dict[str, object],
        core_config: str | None = None,
        core_capability: dict[str, object] | None = None,
    ):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            core = root / "plugins/codexy"
            devtools = root / "plugins/codexy-devtools"
            scripts = root / ".github/scripts"
            for plugin, manifest in (
                (core, core_manifest),
                (devtools, {"name": "codexy-devtools", "supportedPlatforms": []}),
            ):
                (plugin / ".codex-plugin").mkdir(parents=True)
                (plugin / ".codex-plugin/plugin.json").write_text(
                    json.dumps(manifest), encoding="utf-8"
                )
            if core_config is not None:
                (core / ".mcp.json").write_text(core_config, encoding="utf-8")
            if core_capability is not None:
                (core / ".codex-plugin/mcp-capability.json").write_text(
                    json.dumps(core_capability), encoding="utf-8"
                )
            scripts.mkdir(parents=True)
            for name in ("smoke-registered-mcp.ps1", "mcp-smoke-support.ps1"):
                shutil.copy2(REPOSITORY / ".github/scripts" / name, scripts / name)
            environment = os.environ.copy()
            environment["RUNNER_TEMP"] = str(root / "runner-temp")
            return subprocess.run(
                [
                    shutil.which("pwsh") or "pwsh",
                    "-NoProfile",
                    "-NonInteractive",
                    "-File",
                    str(scripts / "smoke-registered-mcp.ps1"),
                ],
                cwd=root,
                env=environment,
                capture_output=True,
                text=True,
                check=False,
            )

    def test_no_mcp_core_is_reported_and_devtools_checks_are_reached(self) -> None:
        result = self._run_smoke(
            {"name": "codexy", "supportedPlatforms": SUPPORTED_PLATFORMS},
            core_capability={
                "schema": "codexy.mcp-capability.v1",
                "component": "core",
                "servers": [],
            },
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        rows = [json.loads(line) for line in result.stdout.splitlines() if line]
        self.assertEqual(rows[0]["status"], "not-registered")
        self.assertEqual([row["plugin"] for row in rows[1:]], ["codexy-devtools"] * 2)

    def test_no_mcp_core_rejects_residual_configuration(self) -> None:
        result = self._run_smoke(
            {"name": "codexy", "supportedPlatforms": SUPPORTED_PLATFORMS},
            "{}",
            {
                "schema": "codexy.mcp-capability.v1",
                "component": "core",
                "servers": [],
            },
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("residual MCP configuration", result.stderr + result.stdout)

    def test_no_mcp_core_requires_a_valid_explicit_contract(self) -> None:
        result = self._run_smoke(
            {"name": "codexy", "supportedPlatforms": SUPPORTED_PLATFORMS},
            core_capability={
                "schema": "codexy.mcp-capability.v2",
                "component": "core",
                "servers": [],
            },
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "invalid no-MCP capability contract", result.stderr + result.stdout
        )

    def test_declared_core_registration_still_requires_its_configuration(self) -> None:
        result = self._run_smoke(
            {
                "name": "codexy",
                "supportedPlatforms": SUPPORTED_PLATFORMS,
                "mcpServers": "./.mcp.json",
            }
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(".mcp.json", result.stderr + result.stdout)


if __name__ == "__main__":
    unittest.main()
