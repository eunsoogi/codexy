"""Host-command helpers shared by GitHub native hook integration cases."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins" / "codexy-github"


class GithubNativeHookSupport:
    @staticmethod
    def _host(environment: dict[str, str], *arguments: str) -> dict[str, object]:
        result = subprocess.run(
            ["codex", *arguments, "--json"],
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode:
            raise AssertionError(
                f"codex {' '.join(arguments)} failed:\n{result.stdout}{result.stderr}"
            )
        return json.loads(result.stdout)

    @staticmethod
    def _assert_enabled_plugins(
        inventory: dict[str, object], expected: set[str]
    ) -> None:
        installed = inventory.get("installed")
        if not isinstance(installed, list):
            raise AssertionError(f"missing installed plugin inventory: {inventory}")
        enabled = {
            entry.get("pluginId")
            for entry in installed
            if isinstance(entry, dict) and entry.get("enabled") is True
        }
        assert enabled == expected, enabled
