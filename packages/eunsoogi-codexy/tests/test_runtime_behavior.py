import contextlib
import io
import json
import tarfile
import tempfile
import unittest
import urllib.request
import zipfile
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import package, runtime
from codexy_runtime_tools.cache import runtime_cache_key
from codexy_runtime_tools.package import (
    _GithubRedirectHandler,
    _artifact_package,
    _safe_extract_tar,
    _safe_extract_zip,
    acquire_package,
)


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

    def test_empty_explicit_package_path_requires_a_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / ".codex-plugin" / "plugin.json"
            manifest.parent.mkdir()
            manifest.write_text('{"version":"1.2.1"}', encoding="utf-8")
            with (
                mock.patch.dict("os.environ", {"CODEXY_RUNTIME_PACKAGE_PATH": ""}, clear=True),
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

    def test_artifacts_skip_invalid_metadata_and_foreign_repositories(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            api = "https://api.github.com/repos/eunsoogi/codexy/actions/artifacts"

            def download(url: str, destination: Path, token: str = "") -> None:
                if url == api:
                    destination.write_text(json.dumps({"artifacts": [
                        {"expired": False, "workflow_run": None},
                        {"expired": False, "workflow_run": "main"},
                        {"expired": False, "workflow_run": {"head_branch": "main", "head_repository_id": 1}, "archive_download_url": "https://api.github.com/fork.zip"},
                        {"expired": False, "workflow_run": {"head_branch": "main", "head_repository_id": 1_269_350_143}, "archive_download_url": "https://api.github.com/valid.zip"},
                    ]}), encoding="utf-8")
                else:
                    with zipfile.ZipFile(destination, "w") as archive:
                        archive.writestr("codexy-marketplace-plugin.tar.gz", b"package")

            with (
                mock.patch("codexy_runtime_tools.package._github_token_for", return_value=""),
                mock.patch("codexy_runtime_tools.package._download", side_effect=download),
            ):
                self.assertEqual(_artifact_package(api, root), root / "artifact" / "codexy-marketplace-plugin.tar.gz")

    def test_truncated_archives_fail_with_runtime_diagnostics(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "truncated.tar.gz"
            with tarfile.open(archive, "w:gz") as packaged:
                member = tarfile.TarInfo("plugins/codexy/plugin.json")
                member.size = 1
                packaged.addfile(member, io.BytesIO(b"x"))
            archive.write_bytes(archive.read_bytes()[:10])
            with self.assertRaisesRegex(ValueError, "invalid runtime package archive"):
                _safe_extract_tar(archive, root / "tar")
            zipped = root / "malformed.zip"
            zipped.write_bytes(b"not a zip archive")
            with self.assertRaisesRegex(ValueError, "invalid artifact archive"):
                _safe_extract_zip(zipped, root / "zip")


if __name__ == "__main__":
    unittest.main()
