import contextlib
import io
import json
import tarfile
import tempfile
import unittest
import urllib.request
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import package, runtime
from codexy_runtime_tools.cache import runtime_cache_key
from codexy_runtime_tools.package import _GithubRedirectHandler, _safe_extract_tar, acquire_package


class Executed(BaseException):
    pass


def configuration(root: Path, **overrides: object) -> runtime.Configuration:
    plugin_root = root / "plugin root 유니코드"
    manifest = plugin_root / ".codex-plugin" / "plugin.json"
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(json.dumps({"version": "1.2.1"}), encoding="utf-8")
    values: dict[str, object] = {
        "server": "lsp", "plugin_root": plugin_root, "arguments": ["--stdio"],
        "platform": "linux-x86_64", "manifest": manifest, "release": "1.2.1",
        "runtime_name": "codexy-mcp-lsp-linux-x86_64.bin", "package_path": "",
        "package_url": "https://example.test/package.tar.gz", "artifacts_api": "",
        "package_override": False, "package_sha256": "",
        "git_repository": "https://example.test/codexy.git", "git_ref": "a" * 40,
        "offline": False, "git_fallback": False,
    }
    values.update(overrides)
    return runtime.Configuration(**values)  # type: ignore[arg-type]


def install_paths(config: runtime.Configuration, cache: Path) -> tuple[Path, Path]:
    source = (
        "\n".join(("package-override", config.package_path, config.package_url, config.artifacts_api, config.package_sha256))
        if config.package_override else "\n".join(("package-default", config.package_sha256))
    )
    key = runtime_cache_key(
        manifest=config.manifest, package_override=config.package_override,
        identity=[config.git_repository, config.git_ref, config.platform, runtime.PROTOCOL, source, "codexy-mcp-lsp"],
    )
    root = cache / key
    return root / "bin" / "codexy-mcp-lsp", root / "plugin.json"


class RuntimeBehaviorTests(unittest.TestCase):
    def test_matching_cached_runtime_reuses_offline_cache(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = configuration(root, offline=True)
            cache = root / "cache"
            installed, marker = install_paths(config, cache)
            installed.parent.mkdir(parents=True)
            installed.write_text("#!/bin/sh\n", encoding="utf-8")
            installed.chmod(0o755)
            marker.write_text('{"version":"1.2.1"}', encoding="utf-8")
            with (
                mock.patch.object(runtime, "_cache_root", return_value=cache),
                mock.patch.object(runtime, "install_package") as acquire,
                mock.patch.object(runtime, "execute", side_effect=Executed) as execute,
                self.assertRaises(Executed),
            ):
                runtime.run(config)
            acquire.assert_not_called()
            execute.assert_called_once_with(installed, ["--stdio"], {"CODEXY_PLUGIN_ROOT": str(config.plugin_root)})

    def test_offline_without_cache_fails_visibly(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            error = io.StringIO()
            with (
                mock.patch.object(runtime, "_cache_root", return_value=root / "cache"),
                contextlib.redirect_stderr(error),
                self.assertRaisesRegex(SystemExit, "127"),
            ):
                runtime.run(configuration(root, offline=True))
            self.assertIn("offline mode has no cached or bundled runtime", error.getvalue())

    def test_explicit_package_source_requires_a_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / ".codex-plugin" / "plugin.json"
            manifest.parent.mkdir()
            manifest.write_text('{"version":"1.2.1"}', encoding="utf-8")
            with (
                mock.patch.dict("os.environ", {"CODEXY_RUNTIME_PACKAGE_PATH": str(root / "package")}, clear=True),
                self.assertRaisesRegex(SystemExit, "127"),
            ):
                runtime.Configuration.load("lsp", root, [])

    def test_explicit_package_digest_must_match(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source.tar.gz"
            source.write_bytes(b"not the expected package")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                acquire_package(path=str(source), url="", artifacts_api="", expected_sha256="0" * 64, work=root / "work")

    def test_tar_symlinks_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "package.tar.gz"
            member = tarfile.TarInfo("plugins/codexy/runtime/link")
            member.type = tarfile.SYMTYPE
            member.linkname = "../../../../outside"
            with tarfile.open(archive, "w:gz") as packaged:
                packaged.addfile(member, io.BytesIO())
            with self.assertRaisesRegex(ValueError, "link"):
                _safe_extract_tar(archive, root / "extract")

    def test_cross_host_redirect_drops_authorization(self) -> None:
        request = urllib.request.Request(
            "https://api.github.com/repos/eunsoogi/codexy/actions/artifacts/1/zip",
            headers={"Authorization": "Bearer secret"},
        )
        redirected = _GithubRedirectHandler().redirect_request(
            request, None, 302, "Found", {}, "https://objects.example.test/artifact.zip"
        )
        self.assertIsNotNone(redirected)
        self.assertIsNone(redirected.get_header("Authorization"))


if __name__ == "__main__":
    unittest.main()
