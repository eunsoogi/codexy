from __future__ import annotations

import os
import json
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools.pre_session import _official_install, _run, run_pre_session

try:
    from .pre_session_support import (
        commands,
        installed,
        make_plugin,
        marketplace,
        respond,
        wrong_marketplace,
    )
except ImportError:
    from pre_session_support import (
        commands,
        installed,
        make_plugin,
        marketplace,
        respond,
        wrong_marketplace,
    )


class PreSessionSafetyTests(unittest.TestCase):
    def test_codex_subprocess_ignores_inherited_git_redirect_configuration(self) -> None:
        inherited = {
            "GIT_CONFIG_GLOBAL": "/attacker/config",
            "GIT_CONFIG_NOSYSTEM": "0",
            "GIT_CONFIG_COUNT": "1",
            "GIT_CONFIG_KEY_0": "url.https://attacker.invalid/.insteadOf",
            "GIT_CONFIG_VALUE_0": "https://github.com/",
        }
        expected_home = Path("/private/tmp/codexy-home")
        with mock.patch.dict(os.environ, {**inherited, "CODEX_HOME": "/wrong-home"}):
            result = _run(["/usr/bin/env"], expected_home)
        environment = dict(
            line.split("=", 1) for line in result.stdout.splitlines() if "=" in line
        )
        self.assertEqual(environment["GIT_CONFIG_GLOBAL"], os.devnull)
        self.assertEqual(environment["GIT_CONFIG_NOSYSTEM"], "1")
        self.assertEqual(environment["GIT_CONFIG_COUNT"], "0")
        self.assertEqual(environment["CODEX_HOME"], str(expected_home))

    def test_wrong_or_ambiguous_marketplace_stops_before_refresh(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marketplace_root = root / "marketplace"
            marketplace_root.mkdir()
            variants = (
                [wrong_marketplace(marketplace_root)],
                [marketplace(marketplace_root), wrong_marketplace(marketplace_root)],
            )
            for marketplaces in variants:
                with self.subTest(marketplaces=marketplaces):
                    calls: list[tuple[str, ...]] = []

                    def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                        calls.append(tuple(command))
                        return subprocess.CompletedProcess(
                            command,
                            0,
                            json.dumps({"marketplaces": marketplaces}),
                            "",
                        )

                    with self.assertRaisesRegex(
                        ValueError,
                        "exactly one official Codexy marketplace",
                    ):
                        run_pre_session(
                            root / "home/.codex",
                            codex=Path("/trusted/codex"),
                            runner=runner,
                            synchronize=lambda *_: self.fail("must not synchronize"),
                            package_version="1.2.2",
                        )

                    self.assertEqual(calls, [commands()[0]])

    def test_duplicate_or_conflicting_enabled_installs_stop_before_projection(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marketplace_root = root / "marketplace"
            plugin = make_plugin(marketplace_root / "plugins/codexy")
            variants = [
                [installed(plugin), installed(plugin)],
                [
                    installed(plugin),
                    {
                        **installed(plugin),
                        "marketplaceSource": wrong_marketplace(marketplace_root)[
                            "marketplaceSource"
                        ],
                    },
                ],
            ]
            for installed_plugins in variants:
                calls: list[tuple[str, ...]] = []
                with self.subTest(installed=installed_plugins), self.assertRaisesRegex(
                    ValueError,
                    "enabled official Codexy install",
                ):
                    run_pre_session(
                        root / "home/.codex",
                        codex=Path("/trusted/codex"),
                        runner=lambda command: respond(
                            command,
                            calls,
                            installed_plugins,
                            [],
                            marketplace_root,
                        ),
                        synchronize=lambda *_: self.fail("must not synchronize"),
                        package_version="1.2.2",
                    )

                self.assertEqual(calls, commands()[:2])

    def test_official_looking_plugin_outside_marketplace_root_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marketplace_root = root / "marketplace"
            marketplace_root.mkdir()
            plugin = make_plugin(root / "outside-plugin")

            with self.assertRaisesRegex(ValueError, "marketplace root"):
                _official_install(
                    {"installed": [installed(plugin)]},
                    marketplace_root,
                    "1.2.2",
                )

    def test_plugin_version_must_match_getcodexy_distribution(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            marketplace_root = Path(temporary) / "marketplace"
            plugin = make_plugin(marketplace_root / "plugins/codexy")

            with self.assertRaisesRegex(ValueError, "getcodexy distribution"):
                _official_install(
                    {"installed": [installed(plugin)]},
                    marketplace_root,
                    "9.9.9",
                )

    def test_symlinked_plugin_root_is_rejected_before_projection(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls: list[tuple[str, ...]] = []
            plugin = make_plugin(root / "actual plugin")
            marketplace_root = root / "marketplace"
            link = marketplace_root / "plugins/codexy"
            link.parent.mkdir(parents=True)
            try:
                os.symlink(plugin, link, target_is_directory=True)
            except (NotImplementedError, OSError) as error:
                self.skipTest(f"symlinks unavailable: {error}")

            with self.assertRaisesRegex(ValueError, "symlink|real"):
                run_pre_session(
                    root / "home/.codex",
                    codex=Path("/trusted/codex"),
                    runner=lambda command: respond(
                        command,
                        calls,
                        [],
                        [installed(link)],
                        marketplace_root,
                    ),
                    synchronize=lambda *_: self.fail("must not synchronize"),
                    package_version="1.2.2",
                )

            self.assertEqual(calls, commands())

    def test_terminal_command_failure_does_not_partially_synchronize(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            calls: list[tuple[str, ...]] = []
            marketplace_root = root / "marketplace"
            marketplace_root.mkdir()

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                if command == list(commands()[2]):
                    calls.append(tuple(command))
                    return subprocess.CompletedProcess(command, 1, "", "failed")
                return respond(command, calls, [], [], marketplace_root)

            with self.assertRaisesRegex(RuntimeError, "marketplace upgrade failed"):
                run_pre_session(
                    root / "home/.codex",
                    codex=Path("/trusted/codex"),
                    runner=runner,
                    synchronize=lambda *_: self.fail("must not synchronize"),
                    package_version="1.2.2",
                )

            self.assertEqual(calls, commands()[:3])

    def test_root_installer_is_pinned_and_readmes_publish_installer_only(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        installer = repository / "install"
        command = "uvx --from getcodexy==1.2.2 codexy-update --pre-session"

        self.assertTrue(installer.is_file())
        self.assertTrue(installer.stat().st_mode & stat.S_IXUSR)
        self.assertIn(command, installer.read_text(encoding="utf-8"))
        for readme in (repository / "README.md", repository / "README.ko.md"):
            text = readme.read_text(encoding="utf-8")
            self.assertIn("/install", text)
            self.assertIn("chmod +x install && ./install", text)
            self.assertNotIn("uvx --from", text)
            self.assertNotIn("python3 -c", text)
            self.assertNotIn("bootstrap-codexy-agents", text)
            self.assertNotIn("SessionStart", text)
            self.assertNotIn("codex plugin marketplace add", text)
            self.assertNotIn("codex plugin add", text)
            self.assertNotIn("codex plugin list", text)
            self.assertNotIn("codex mcp list", text)
