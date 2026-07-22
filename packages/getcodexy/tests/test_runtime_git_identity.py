"""Git-fallback provenance and cache isolation regression coverage."""

import hashlib
import json
import os
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import runtime
import test_runtime_source_identity as source_identity_tests


class Executed(BaseException):
    pass


class RuntimeGitIdentityTests(unittest.TestCase):
    def test_git_fallback_has_an_exact_independent_cache_identity_and_marker(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_identity_tests.RuntimeSourceIdentityTests._write_selected_release(root)
            cache = root / "cache"
            base = runtime.Configuration.load("lsp", root, [])
            config = replace(base, git_fallback=True)

            def install_git(_config: object, install_root: Path, installed: Path) -> None:
                installed.parent.mkdir(parents=True)
                installed.write_bytes(b"pinned Git runtime")
                installed.chmod(0o755)
                (install_root / "plugin.json").write_text('{"version":"1.3.0"}', encoding="utf-8")

            environment = {"CODEXY_RUNTIME_CACHE_DIR": str(cache)}
            with mock.patch.dict(os.environ, environment, clear=True), \
                 mock.patch.object(runtime, "install_package", side_effect=RuntimeError("missing public package")), \
                 mock.patch.object(runtime, "install_git", side_effect=install_git), \
                 mock.patch.object(runtime, "_execute", side_effect=Executed), \
                 self.assertRaises(Executed):
                runtime.run(config)
            marker = next(cache.rglob("runtime-marker.json"))
            document = json.loads(marker.read_text(encoding="utf-8"))
            self.assertEqual(document["schema"], "codexy-runtime-git-marker/v1")
            self.assertEqual(document["identity"]["mode"], "git-fallback")
            self.assertEqual(document["identity"]["source"], {
                "repository": config.git_repository, "commit": config.git_ref,
            })

            with mock.patch.dict(os.environ, environment, clear=True), \
                 mock.patch.object(runtime, "install_package") as package_install, \
                 mock.patch.object(runtime, "install_git") as git_install, \
                 mock.patch.object(runtime, "_execute", side_effect=Executed), \
                 self.assertRaises(Executed):
                runtime.run(replace(config, offline=True))
            package_install.assert_not_called()
            git_install.assert_not_called()

            cases = (
                ("different-ref", replace(config, git_ref="b" * 40)),
                ("different-repository", replace(config, git_repository="https://example.test/other")),
                ("different-platform", replace(config, platform=("linux-x86_64" if config.platform == "darwin-arm64" else "darwin-arm64"))),
                ("different-server", replace(config, server="codegraph", runtime_name="codexy-mcp-codegraph-linux-x86_64.bin")),
            )
            for label, mismatched in cases:
                with self.subTest(case=label), mock.patch.dict(os.environ, environment, clear=True), \
                     mock.patch.object(runtime, "_execute") as execute, self.assertRaisesRegex(SystemExit, "127"):
                    runtime.run(replace(mismatched, offline=True))
                execute.assert_not_called()

            for corruption in ("missing", "corrupt", "package-marker"):
                if corruption == "missing":
                    marker.unlink()
                elif corruption == "corrupt":
                    marker.write_text("not json", encoding="utf-8")
                else:
                    marker.write_text(json.dumps(base.source_identity.marker(
                        platform=config.platform, server=config.server,
                        binary_sha256=hashlib.sha256(b"pinned Git runtime").hexdigest(),
                    )), encoding="utf-8")
                with self.subTest(marker=corruption), mock.patch.dict(os.environ, environment, clear=True), \
                     mock.patch.object(runtime, "_execute") as execute, self.assertRaisesRegex(SystemExit, "127"):
                    runtime.run(replace(config, offline=True))
                execute.assert_not_called()
                marker.write_text(json.dumps(document), encoding="utf-8")

    def test_failed_git_fallback_never_admits_partial_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_identity_tests.RuntimeSourceIdentityTests._write_selected_release(root)
            cache = root / "cache"
            config = replace(runtime.Configuration.load("lsp", root, []), git_fallback=True)

            def partial_install(_config: object, _install_root: Path, installed: Path) -> None:
                installed.parent.mkdir(parents=True)
                installed.write_bytes(b"partial")
                installed.chmod(0o755)
                raise RuntimeError("cargo failed")

            environment = {"CODEXY_RUNTIME_CACHE_DIR": str(cache)}
            with mock.patch.dict(os.environ, environment, clear=True), \
                 mock.patch.object(runtime, "install_package", side_effect=RuntimeError("missing public package")), \
                 mock.patch.object(runtime, "install_git", side_effect=partial_install), \
                 self.assertRaisesRegex(SystemExit, "127"):
                runtime.run(config)
            self.assertEqual(list(cache.rglob("runtime-marker.json")), [])
            with mock.patch.dict(os.environ, environment, clear=True), \
                 mock.patch.object(runtime, "_execute") as execute, self.assertRaisesRegex(SystemExit, "127"):
                runtime.run(replace(config, offline=True))
            execute.assert_not_called()


if __name__ == "__main__":
    unittest.main()
