"""Host-independent controls for Windows component-integrity fallback."""

from __future__ import annotations

import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from codexy_runtime_tools.component_integrity import (
    _has_windows_reparse_point,
    frozen_component,
)


REPOSITORY = Path(__file__).resolve().parents[3]


class ComponentIntegrityWindowsTests(unittest.TestCase):
    def test_windows_fallback_freezes_a_regular_component(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            component = Path(temporary) / "codexy-github"
            shutil.copytree(REPOSITORY / "plugins/codexy-github", component)
            with mock.patch(
                "codexy_runtime_tools.component_integrity._uses_windows_directory_fallback",
                return_value=True,
            ):
                with frozen_component(component, "codexy-github") as frozen:
                    self.assertTrue((frozen / "agents/codexy-weaver.toml").is_file())

    def test_reparse_point_attribute_is_rejected_without_a_windows_host(self) -> None:
        metadata = SimpleNamespace(
            st_mode=stat.S_IFREG,
            st_file_attributes=getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400),
        )
        self.assertTrue(_has_windows_reparse_point(metadata))

    def test_windows_fallback_rejects_a_symlinked_ancestor(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            component = Path(temporary) / "codexy-github"
            shutil.copytree(REPOSITORY / "plugins/codexy-github", component)
            agents = component / "agents"
            moved = component / "trusted-agents"
            agents.rename(moved)
            agents.symlink_to(moved, target_is_directory=True)
            with mock.patch(
                "codexy_runtime_tools.component_integrity._uses_windows_directory_fallback",
                return_value=True,
            ):
                with self.assertRaisesRegex(ValueError, "link|reparse"):
                    with frozen_component(component, "codexy-github"):
                        pass

    def test_windows_checkout_keeps_packaged_skill_bridge_integrity(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary) / "repository"
            repository.mkdir()
            shutil.copy2(REPOSITORY / ".gitattributes", repository)
            shutil.copytree(
                REPOSITORY / "plugins/codexy-github",
                repository / "plugins/codexy-github",
            )
            self._git(repository, "init", "--quiet")
            self._git(repository, "config", "user.email", "lint@example.test")
            self._git(repository, "config", "user.name", "Lint Test")
            self._git(repository, "add", ".")
            self._git(repository, "commit", "--quiet", "-m", "test fixture")
            self._git(repository, "config", "core.autocrlf", "true")
            self._git(repository, "reset", "--hard", "HEAD")

            bridge = (
                repository
                / "plugins/codexy-github/skills/git-workflow/scripts/bootstrap-codexy-github-agent"
            )
            self.assertNotIn(b"\r\n", bridge.read_bytes())
            with frozen_component(
                repository / "plugins/codexy-github", "codexy-github"
            ):
                pass

    @staticmethod
    def _git(repository: Path, *args: str) -> None:
        subprocess.run(["git", *args], cwd=repository, check=True, capture_output=True)


if __name__ == "__main__":
    unittest.main()
