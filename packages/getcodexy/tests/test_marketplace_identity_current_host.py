from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.marketplace_identity import require_pinned_registration


class CurrentHostMarketplaceIdentityTests(unittest.TestCase):
    def test_exact_tag_checkout_without_legacy_metadata_is_accepted(self) -> None:
        with _checkout() as state:
            require_pinned_registration(state.home, state.marketplace, state.tag)

    def test_checkout_drift_without_legacy_metadata_is_rejected(self) -> None:
        with _checkout() as state:
            _git(state.marketplace, "checkout", "-q", "--detach", "main")

            with self.assertRaisesRegex(
                RuntimeError, "checkout revision is outside the expected release tag"
            ):
                require_pinned_registration(state.home, state.marketplace, state.tag)


class _checkout:
    def __enter__(self) -> "_checkout":
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        self.home = self.root / "home"
        self.marketplace = self.root / "marketplace"
        self.home.mkdir()
        self.marketplace.mkdir()
        self.tag = "v1.6.2"
        (self.home / "config.toml").write_text(
            f'[marketplaces.codexy]\nref = "{self.tag}"\n', encoding="utf-8"
        )
        _git(self.marketplace, "init", "-q")
        _git(self.marketplace, "branch", "-M", "main")
        _git(self.marketplace, "config", "user.name", "fixture")
        _git(self.marketplace, "config", "user.email", "fixture@example.invalid")
        (self.marketplace / "release-marker").write_text("release", encoding="utf-8")
        _git(self.marketplace, "add", "release-marker")
        _git(self.marketplace, "commit", "-qm", "release")
        _git(self.marketplace, "tag", self.tag)
        (self.marketplace / "main-marker").write_text("main", encoding="utf-8")
        _git(self.marketplace, "add", "main-marker")
        _git(self.marketplace, "commit", "-qm", "main drift")
        _git(self.marketplace, "checkout", "-q", "--detach", self.tag)
        self.assert_no_legacy_metadata()
        return self

    def assert_no_legacy_metadata(self) -> None:
        if (self.marketplace / ".codex-marketplace-install.json").exists():
            raise AssertionError(
                "current-host fixture unexpectedly has legacy metadata"
            )

    def __exit__(self, *_: object) -> None:
        self.temporary.cleanup()


def _git(root: Path, *arguments: str) -> None:
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)


if __name__ == "__main__":
    unittest.main()
