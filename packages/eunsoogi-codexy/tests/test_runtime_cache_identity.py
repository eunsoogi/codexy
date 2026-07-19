import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import runtime
from runtime_fixture import configuration, install_paths


class Executed(BaseException):
    pass


class RuntimeCacheIdentityTests(unittest.TestCase):
    def config(self, root: Path, **overrides: object) -> runtime.Configuration:
        return configuration(root, **overrides)

    def seed_cached_runtime(self, config: runtime.Configuration, cache: Path) -> Path:
        installed, marker = install_paths(config, cache)
        installed.parent.mkdir(parents=True)
        installed.write_text("#!/bin/sh\n", encoding="utf-8")
        installed.chmod(0o755)
        marker.write_text(json.dumps({"version": "1.2.1"}), encoding="utf-8")
        return installed

    def test_matching_cached_runtime_is_reused_offline_without_acquisition(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = self.config(root, offline=True)
            cache = root / "cache"
            installed = self.seed_cached_runtime(config, cache)
            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "install_package") as acquire,
                mock.patch.object(runtime, "execute", side_effect=Executed) as execute,
                self.assertRaises(Executed),
            ):
                runtime.run(config)
            acquire.assert_not_called()
            execute.assert_called_once_with(
                installed,
                ["--stdio"],
                {"CODEXY_PLUGIN_ROOT": str(config.plugin_root)},
            )

    def test_default_digest_does_not_reuse_unsigned_cached_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cache = root / "cache"
            unsigned = self.config(root, offline=True)
            digest_checked = self.config(root, offline=True, package_sha256="0" * 64)
            self.seed_cached_runtime(unsigned, cache)
            error = io.StringIO()

            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "execute", side_effect=Executed),
                contextlib.redirect_stderr(error),
                self.assertRaisesRegex(SystemExit, "127"),
            ):
                runtime.run(digest_checked)

            self.assertIn("offline mode has no cached", error.getvalue())

    def test_default_digest_cache_reuses_case_equivalent_checksum(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cache = root / "cache"
            plugin_root = self.config(root).plugin_root
            with mock.patch.dict(
                "os.environ",
                {"CODEXY_RUNTIME_PACKAGE_SHA256": "a" * 64, "UV_OFFLINE": "1"},
                clear=True,
            ):
                lowercase = runtime.Configuration.load("lsp", plugin_root, ["--stdio"])
            with mock.patch.dict(
                "os.environ",
                {"CODEXY_RUNTIME_PACKAGE_SHA256": "A" * 64, "UV_OFFLINE": "1"},
                clear=True,
            ):
                uppercase = runtime.Configuration.load("lsp", plugin_root, ["--stdio"])
            installed = self.seed_cached_runtime(lowercase, cache)

            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "execute", side_effect=Executed) as execute,
                self.assertRaises(Executed),
            ):
                runtime.run(uppercase)

            execute.assert_called_once_with(
                installed,
                ["--stdio"],
                {"CODEXY_PLUGIN_ROOT": str(uppercase.plugin_root)},
            )


if __name__ == "__main__":
    unittest.main()
