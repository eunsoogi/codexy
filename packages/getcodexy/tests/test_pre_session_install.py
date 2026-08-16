from __future__ import annotations

import contextlib
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.pre_session import official_marketplace_root, run_pre_session
from codexy_runtime_tools.updater import SyncResult
from codexy_runtime_tools.version_lock import default_package_version

try:
    from .pre_session_support import (
        commands,
        fresh_marketplace_commands,
        installed,
        make_plugin,
        marketplace,
        respond,
    )
except ImportError:
    from pre_session_support import (
        commands,
        fresh_marketplace_commands,
        installed,
        make_plugin,
        marketplace,
        respond,
    )


class PreSessionInstallTests(unittest.TestCase):
    def test_default_marketplace_ref_uses_the_packaged_lockfile_version(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            registered = False
            calls: list[tuple[str, ...]] = []

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                nonlocal registered
                calls.append(tuple(command))
                if command[1:4] == ["plugin", "marketplace", "list"]:
                    payload: object = {
                        "marketplaces": [marketplace(root)] if registered else []
                    }
                else:
                    registered = True
                    payload = {"ok": True}
                return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

            self.assertEqual(
                official_marketplace_root(Path("/trusted/codex"), runner),
                root.resolve(),
            )
            self.assertIn(
                (
                    "/trusted/codex",
                    "plugin",
                    "marketplace",
                    "add",
                    "eunsoogi/codexy",
                    "--ref",
                    f"v{default_package_version()}",
                    "--json",
                ),
                calls,
            )

    def test_fresh_marketplace_is_added_before_preflight_and_sync(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls: list[tuple[str, ...]] = []
            synchronized: list[tuple[Path, Path, str]] = []
            marketplace_root = root / "marketplace"
            plugin = make_plugin(marketplace_root / "plugins/codexy")
            expected = [
                *fresh_marketplace_commands(),
                commands()[1],
                *commands()[5:],
            ]

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                calls.append(tuple(command))
                if len(calls) == 1:
                    payload: object = {"marketplaces": []}
                elif command[1:4] == ["plugin", "marketplace", "add"]:
                    ref = command[command.index("--ref") + 1]
                    home = root / "home/.codex"
                    home.mkdir(parents=True, exist_ok=True)
                    (home / "config.toml").write_text(
                        f'[marketplaces.codexy]\nref = "{ref}"\n', encoding="utf-8"
                    )
                    payload = {"ok": True}
                elif command[1:4] == ["plugin", "marketplace", "list"]:
                    payload = {"marketplaces": [marketplace(marketplace_root)]}
                elif len(calls) == 4:
                    payload = {"installed": []}
                else:
                    payload = {"installed": [installed(plugin)]}
                return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

            result = run_pre_session(
                root / "home/.codex",
                codex=Path("/trusted/codex"),
                runner=runner,
                synchronize=lambda root, home, mode: (
                    synchronized.append((root, home, mode))
                    or SyncResult(
                        mode,
                        "ready",
                        "codexy",
                        str(root),
                        str(home),
                        False,
                        False,
                        (),
                    )
                ),
                package_version="1.2.2",
            )

            self.assertFalse(result.changed)
            self.assertEqual(calls, expected)
            self.assertEqual([mode for _, _, mode in synchronized], ["check"])

    def test_official_install_refreshes_then_checks_a_current_projection_quietly(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls: list[tuple[str, ...]] = []
            synchronized: list[tuple[Path, Path, str]] = []
            plugin = make_plugin(root / "marketplace/plugins/codexy")

            result, stdout, stderr = self.invoke(
                root,
                calls,
                synchronized,
                plugin,
                check="ready",
            )

            self.assertFalse(result.changed)
            self.assertEqual((stdout, stderr), ("", ""))
            self.assertEqual(calls, commands())
            self.assertEqual(
                synchronized,
                [(plugin.resolve(), (root / "home/.codex").resolve(), "check")],
            )

    def test_stale_projection_is_installed_after_the_check(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls: list[tuple[str, ...]] = []
            synchronized: list[tuple[Path, Path, str]] = []
            plugin = make_plugin(root / "marketplace/plugins/codexy")

            result, _, _ = self.invoke(
                root,
                calls,
                synchronized,
                plugin,
                check="update_required",
                changed=True,
            )

            self.assertTrue(result.changed)
            self.assertEqual(calls, commands())
            self.assertEqual(
                synchronized[-2:],
                [
                    (plugin.resolve(), (root / "home/.codex").resolve(), "check"),
                    (plugin.resolve(), (root / "home/.codex").resolve(), "install"),
                ],
            )

    def invoke(
        self,
        root: Path,
        calls: list[tuple[str, ...]],
        synchronized: list[tuple[Path, Path, str]],
        plugin: Path,
        *,
        check: str,
        changed: bool = False,
    ) -> tuple[object, str, str]:
        home = root / "home/.codex"
        home.mkdir(parents=True)
        (home / "config.toml").write_text(
            '[marketplaces.codexy]\nref = "v1.2.2"\n', encoding="utf-8"
        )

        def synchronize(plugin_root: Path, home: Path, mode: str) -> SyncResult:
            synchronized.append((plugin_root, home, mode))
            return SyncResult(
                mode,
                "completed" if mode == "install" else check,
                "codexy",
                str(plugin_root),
                str(home),
                changed if mode == "install" else False,
                False,
                (),
            )

        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                result = respond(
                    command,
                    calls,
                    [],
                    [installed(plugin)],
                    root / "marketplace",
                )
                if (
                    command[1:4] == ["plugin", "marketplace", "add"]
                    and result.returncode == 0
                ):
                    ref = command[command.index("--ref") + 1]
                    (home / "config.toml").write_text(
                        f'[marketplaces.codexy]\nref = "{ref}"\n', encoding="utf-8"
                    )
                return result

            result = run_pre_session(
                home,
                codex=Path("/trusted/codex"),
                runner=runner,
                synchronize=synchronize,
                package_version="1.2.2",
            )
        return result, stdout.getvalue(), stderr.getvalue()
