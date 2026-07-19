import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import runtime
from codexy_runtime_tools.cache import runtime_cache_key


class Executed(BaseException):
    pass


class RuntimeFlowTests(unittest.TestCase):
    def config(self, root: Path, **overrides: object) -> runtime.Configuration:
        plugin_root = root / "plugin root 유니코드"
        manifest = plugin_root / ".codex-plugin" / "plugin.json"
        manifest.parent.mkdir(parents=True)
        manifest.write_text(json.dumps({"version": "1.2.1"}), encoding="utf-8")
        values: dict[str, object] = {
            "server": "lsp",
            "plugin_root": plugin_root,
            "arguments": ["--stdio"],
            "platform": "linux-x86_64",
            "manifest": manifest,
            "release": "1.2.1",
            "runtime_name": "codexy-mcp-lsp-linux-x86_64.bin",
            "package_path": "",
            "package_url": "https://example.test/package.tar.gz",
            "artifacts_api": "",
            "package_override": False,
            "package_sha256": "",
            "git_repository": "https://example.test/codexy.git",
            "git_ref": "a" * 40,
            "offline": False,
            "git_fallback": False,
        }
        values.update(overrides)
        return runtime.Configuration(**values)  # type: ignore[arg-type]

    def install_paths(self, config: runtime.Configuration, cache: Path) -> tuple[Path, Path]:
        source = (
            "\n".join(
                (
                    "package-override",
                    config.package_path,
                    config.package_url,
                    config.artifacts_api,
                    config.package_sha256,
                )
            )
            if config.package_override
            else "package-default"
        )
        key = runtime_cache_key(
            manifest=config.manifest,
            package_override=config.package_override,
            identity=[
                config.git_repository,
                config.git_ref,
                config.platform,
                runtime.PROTOCOL,
                source,
                f"codexy-mcp-{config.server}",
            ],
        )
        root = cache / key
        return root / "bin" / f"codexy-mcp-{config.server}", root / "plugin.json"

    def seed_cached_runtime(
        self, config: runtime.Configuration, cache: Path, version: str = "1.2.1"
    ) -> Path:
        installed, marker = self.install_paths(config, cache)
        installed.parent.mkdir(parents=True)
        installed.write_text("#!/bin/sh\n", encoding="utf-8")
        installed.chmod(0o755)
        marker.write_text(json.dumps({"version": version}), encoding="utf-8")
        return installed

    def test_clean_acquisition_installs_and_executes_exact_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = self.config(root)
            cache = root / "cache"
            installed, marker = self.install_paths(config, cache)

            def install(*_: object) -> None:
                installed.parent.mkdir(parents=True)
                installed.write_text("runtime", encoding="utf-8")
                installed.chmod(0o755)
                marker.write_text('{"version":"1.2.1"}', encoding="utf-8")

            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "install_package", side_effect=install) as acquire,
                mock.patch.object(runtime, "execute", side_effect=Executed),
                self.assertRaises(Executed),
            ):
                runtime.run(config)
            acquire.assert_called_once()

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
            execute.assert_called_once_with(installed, ["--stdio"])

    def test_stale_marker_reacquires_instead_of_reusing_old_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = self.config(root)
            cache = root / "cache"
            self.seed_cached_runtime(config, cache, version="1.2.0")
            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "install_package") as acquire,
                mock.patch.object(runtime, "execute", side_effect=Executed),
                self.assertRaises(Executed),
            ):
                runtime.run(config)
            acquire.assert_called_once()

    def test_offline_without_cache_fails_visibly(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = self.config(root, offline=True)
            error = io.StringIO()
            with (
                mock.patch.object(runtime, "_cache_root", return_value=root / "cache"),
                contextlib.redirect_stderr(error),
                self.assertRaisesRegex(SystemExit, "127"),
            ):
                runtime.run(config)
            self.assertIn("offline mode has no cached or bundled runtime", error.getvalue())

    def test_failed_release_uses_only_explicit_git_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = self.config(root, git_fallback=True)
            cache = root / "cache"
            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "install_package", side_effect=RuntimeError("release 404")),
                mock.patch.object(runtime, "install_git") as install_git,
                mock.patch.object(runtime, "execute", side_effect=Executed),
                self.assertRaises(Executed),
            ):
                runtime.run(config)
            install_git.assert_called_once()

    def test_failed_release_without_fallback_reports_diagnostic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = self.config(root)
            error = io.StringIO()
            with (
                mock.patch.object(runtime, "_cache_root", return_value=root / "cache"),
                mock.patch.object(runtime, "install_package", side_effect=RuntimeError("release 404")),
                contextlib.redirect_stderr(error),
                self.assertRaisesRegex(SystemExit, "127"),
            ):
                runtime.run(config)
            self.assertIn("exact release package failed: release 404", error.getvalue())

    def test_override_requires_digest_and_skips_marker_comparison(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = self.config(root, package_override=True, package_path=str(root / "package"))
            cache = root / "cache"
            installed, _ = self.install_paths(config, cache)
            installed.parent.mkdir(parents=True)
            installed.write_text("runtime", encoding="utf-8")
            installed.chmod(0o755)
            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "execute", side_effect=Executed) as execute,
                self.assertRaises(Executed),
            ):
                runtime.run(config)
            execute.assert_called_once_with(installed, ["--stdio"])

    def test_explicit_override_without_digest_is_rejected_at_load(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.config(root)
            with (
                mock.patch.dict(
                    "os.environ",
                    {"CODEXY_RUNTIME_PACKAGE_PATH": str(root / "package")},
                    clear=True,
                ),
                self.assertRaisesRegex(SystemExit, "127"),
            ):
                runtime.Configuration.load("lsp", root / "plugin root 유니코드", [])


if __name__ == "__main__":
    unittest.main()
