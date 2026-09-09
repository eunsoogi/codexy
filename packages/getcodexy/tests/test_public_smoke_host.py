"""Exercise the release smoke host through the production hook reader."""

import json
import os
import shutil
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from codexy_runtime_tools.component_hook_activation_host import (
    HookStateError,
    list_hooks,
)


class PublicSmokeHostTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        self.marketplace = root / "marketplace"
        self.home = root / "home"
        self.home.mkdir()
        repository = Path(__file__).resolve().parents[3]
        for name in ("codexy", "codexy-github"):
            shutil.copytree(
                repository / "plugins" / name / "hooks",
                self.marketplace / "plugins" / name / "hooks",
            )
        script = repository / "scripts/fake_public_codex_host.py"
        self.host = root / ("codex.cmd" if os.name == "nt" else "codex")
        if os.name == "nt":
            self.host.write_text(f'@echo off\r\n"{sys.executable}" "{script}" %*\r\n')
        else:
            self.host.write_text(
                f'#!/bin/sh\nexec "{sys.executable}" "{script}" "$@"\n'
            )
            self.host.chmod(0o700)
        self.environment = patch.dict(
            os.environ,
            {
                "CODEXY_MARKETPLACE_ROOT": str(self.marketplace),
                "CODEX_HOME": str(self.home),
                "TARGET_VERSION": "1.7.0",
            },
        )
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def select(self, components: list[str]) -> None:
        (self.home / ".codexy-public-proof.json").write_text(
            json.dumps({"selection": components, "versions": {}})
        )

    def test_lists_real_hooks_only_for_installed_components(self) -> None:
        self.select([])
        self.assertEqual(list_hooks(self.host, self.home), ())
        self.select(["core"])
        rows = list_hooks(self.host, self.home)
        self.assertTrue(rows)
        self.assertEqual({row["pluginId"] for row in rows}, {"codexy@codexy"})
        self.select(["core", "github", "devtools"])
        rows = list_hooks(self.host, self.home)
        self.assertEqual(
            {row["pluginId"] for row in rows}, {"codexy@codexy", "codexy-github@codexy"}
        )
        command_key = "commandWindows" if os.name == "nt" else "command"
        for row in rows:
            definition = json.loads(Path(row["sourcePath"]).read_text())["hooks"]
            commands = {
                hook[command_key].replace(
                    "${PLUGIN_ROOT}", str(Path(row["sourcePath"]).parent.parent)
                )
                for groups in definition.values()
                for group in groups
                for hook in group["hooks"]
            }
            self.assertIn(row["command"], commands)
            self.assertEqual(row["trustStatus"], "trusted")
            self.assertTrue(row["enabled"])

    def test_missing_or_uninspectable_installed_hooks_fail_closed(self) -> None:
        self.select(["core", "github"])
        path = self.marketplace / "plugins/codexy-github/hooks/hooks.json"
        for broken in (None, "not json", '{"hooks":{"UnexpectedEvent":[]}}'):
            with self.subTest(broken=broken):
                if broken is None:
                    path.unlink()
                else:
                    path.write_text(broken)
                with self.assertRaises(HookStateError):
                    list_hooks(self.host, self.home)
