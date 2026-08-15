"""Host-command helpers shared by GitHub native hook integration cases."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins" / "codexy-github"


class GithubNativeHookSupport:
    @staticmethod
    def _admission(
        installed: Path,
        environment: dict[str, str],
        rule: str,
        title: str,
        denied: bool,
    ) -> None:
        GithubNativeHookSupport._admission_payload(
            installed,
            environment,
            rule,
            {
                "tool_name": "mcp__codex_apps__github_create_issue",
                "tool_input": {"title": title},
            },
            denied,
        )

    @staticmethod
    def _admission_payload(
        installed: Path,
        environment: dict[str, str],
        rule: str,
        payload: dict[str, object],
        denied: bool,
    ) -> None:
        GithubNativeHookSupport._admission_raw(
            installed, environment, rule, json.dumps(payload), denied
        )

    @staticmethod
    def _admission_raw(
        installed: Path,
        environment: dict[str, str],
        rule: str,
        payload: str,
        denied: bool,
    ) -> None:
        result = subprocess.run(
            [str(installed / "hooks/codexy-github-admission.sh"), "--rule", rule],
            input=payload,
            text=True,
            capture_output=True,
            env={**environment, "PLUGIN_ROOT": str(installed)},
            check=False,
        )
        if result.returncode:
            raise AssertionError(result.stderr)
        assert ("permissionDecision" in result.stdout) == denied, result.stdout

    @staticmethod
    def _run(path: Path, *arguments: str) -> None:
        result = subprocess.run(
            [str(path), *arguments], text=True, capture_output=True, check=False
        )
        if result.returncode:
            raise AssertionError(f"{path.name} failed:\n{result.stdout}{result.stderr}")

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
