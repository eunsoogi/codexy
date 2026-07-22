"""Runtime source identity boundaries for selected releases and explicit overrides."""

import hashlib
import io
import json
import os
import shutil
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import runtime
from codexy_runtime_tools import package


class Executed(BaseException):
    pass


class RuntimeSourceIdentityTests(unittest.TestCase):
    def test_all_sha_pinned_override_sources_are_independent_and_cache_isolated(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self._write_selected_release(root)
            archive = root / "override.tar.gz"
            self._write_override(archive)
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            for source_name, source_key, source in (
                ("path", "CODEXY_RUNTIME_PACKAGE_PATH", str(archive)),
                ("url", "CODEXY_RUNTIME_PACKAGE_URL", "https://example.test/override.tar.gz"),
                ("artifacts", "CODEXY_RUNTIME_ARTIFACTS_API_URL", "https://api.github.com/repos/eunsoogi/codexy/actions/artifacts"),
            ):
                with self.subTest(source=source_name):
                    cache = root / f"cache-{source_name}"
                    environment = {
                        "CODEXY_RUNTIME_CACHE_DIR": str(cache),
                        source_key: source,
                        "CODEXY_RUNTIME_PACKAGE_SHA256": digest,
                        "CODEXY_RUNTIME_PLATFORM": "linux-x86_64",
                    }
                    download = (lambda _url, destination, _token="": shutil.copyfile(archive, destination))
                    with mock.patch.dict(os.environ, environment, clear=True), \
                         mock.patch.object(package, "_download", side_effect=download), \
                         mock.patch.object(package, "_artifact_package", return_value=archive), \
                         mock.patch.object(runtime, "_execute", side_effect=Executed), \
                         self.assertRaises(Executed):
                        runtime.run(runtime.Configuration.load("lsp", root, []))
                    marker = next(cache.rglob("runtime-marker.json"))
                    self.assertEqual(json.loads(marker.read_text())["identity"]["mode"],
                                     "explicit-override")
                    with mock.patch.dict(os.environ, {**environment, "UV_OFFLINE": "1"}, clear=True), \
                         mock.patch.object(runtime, "_execute", side_effect=Executed), \
                         self.assertRaises(Executed):
                        runtime.run(runtime.Configuration.load("lsp", root, []))
                    with mock.patch.dict(os.environ, {
                        "CODEXY_RUNTIME_CACHE_DIR": str(cache), "UV_OFFLINE": "1",
                        "CODEXY_RUNTIME_PLATFORM": "linux-x86_64",
                    }, clear=True), mock.patch.object(runtime, "_execute") as execute, \
                         self.assertRaisesRegex(SystemExit, "127"):
                        runtime.run(runtime.Configuration.load("lsp", root, []))
                    execute.assert_not_called()

    def test_override_admission_failures_do_not_install_a_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self._write_selected_release(root)
            cases = {
                "missing-runtime": {
                    "plugins/codexy/.codex-plugin/plugin.json": b'{"version":"1"}',
                },
                "casefold-duplicate": {
                    "plugins/codexy/.codex-plugin/plugin.json": b'{"version":"1"}',
                    "plugins/codexy/runtime/codexy-mcp-lsp-linux-x86_64.bin": b"one",
                    "PLUGINS/codexy/runtime/codexy-mcp-lsp-linux-x86_64.bin": b"two",
                },
            }
            for name, files in cases.items():
                with self.subTest(case=name):
                    archive = root / f"{name}.tar.gz"
                    self._write_archive(archive, files)
                    cache = root / f"cache-{name}"
                    environment = {
                        "CODEXY_RUNTIME_CACHE_DIR": str(cache),
                        "CODEXY_RUNTIME_PACKAGE_PATH": str(archive),
                        "CODEXY_RUNTIME_PACKAGE_SHA256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                        "CODEXY_RUNTIME_PLATFORM": "linux-x86_64",
                    }
                    with mock.patch.dict(os.environ, environment, clear=True), \
                         self.assertRaisesRegex(SystemExit, "127"):
                        runtime.run(runtime.Configuration.load("lsp", root, []))
                    self.assertEqual(list(cache.rglob("bin/codexy-mcp-lsp")), [])

    @staticmethod
    def _write_selected_release(root: Path) -> None:
        manifest = root / ".codex-plugin/plugin.json"
        manifest.parent.mkdir()
        manifest.write_text('{"name":"codexy","version":"1.3.0"}', encoding="utf-8")
        selected_binary = b"selected public runtime"
        candidate = {
            "schema": "codexy-runtime-candidate/v1",
            "source": {"repository": runtime.REPOSITORY, "commit": "a" * 40},
            "artifact": {"tag": "runtime-candidate-1.3.0"},
            "compatibility": {
                "bootstrapApi": 1,
                "pluginRuntimeApi": 1,
                "transport": runtime.PROTOCOL,
                "mcpProtocol": "2024-11-05",
            },
            "platforms": {
                platform: {
                    server: {
                        "path": f"runtime/codexy-mcp-{server}-{platform}.bin",
                        "sha256": hashlib.sha256(selected_binary).hexdigest(),
                    }
                    for server in ("lsp", "codegraph")
                }
                for platform in ("darwin-arm64", "linux-x86_64")
            },
        }
        encoded = json.dumps(candidate, sort_keys=True, separators=(",", ":")).encode()
        release = {
            **candidate,
            "schema": "codexy-runtime-release/v1",
            "state": "candidate-proven",
            "artifact": {
                "tag": candidate["artifact"]["tag"],
                "url": f"{runtime.REPOSITORY}/releases/download/runtime-candidate-1.3.0/codexy-marketplace-plugin.tar.gz",
                "sha256": "b" * 64,
                "payloadManifestSha256": hashlib.sha256(encoded).hexdigest(),
            },
        }
        (root / "runtime-release.json").write_text(json.dumps(release), encoding="utf-8")

    @staticmethod
    def _write_override(archive: Path) -> None:
        files = {
            "plugins/codexy/.codex-plugin/plugin.json": b'{"name":"codexy","version":"77.0.0"}',
            "plugins/codexy/runtime/codexy-mcp-lsp-linux-x86_64.bin": b"authorized override runtime",
        }
        RuntimeSourceIdentityTests._write_archive(archive, files)

    @staticmethod
    def _write_archive(archive: Path, files: dict[str, bytes]) -> None:
        with tarfile.open(archive, "w:gz") as packaged:
            for name, data in files.items():
                member = tarfile.TarInfo(name)
                member.size = len(data)
                packaged.addfile(member, io.BytesIO(data))


if __name__ == "__main__":
    unittest.main()
