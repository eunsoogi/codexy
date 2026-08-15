"""Runtime package roots stay aligned with their selected release contract."""

import hashlib
import io
import json
import os
import shutil
import tarfile
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from codexy_runtime_tools import contract, package, runtime
from codexy_runtime_tools.installer import install_package
from codexy_runtime_tools.source import RuntimeSourceIdentity


REPOSITORY = "https://github.com/eunsoogi/codexy"
COMMIT = "a" * 40
BINARIES = {"lsp": b"lsp runtime", "codegraph": b"codegraph runtime"}


class Executed(BaseException):
    pass


def encoded(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def candidate() -> dict[str, object]:
    platforms = {
        platform: {
            server: {
                "path": f"runtime/codexy-mcp-{server}-{platform}.bin",
                "sha256": hashlib.sha256(binary).hexdigest(),
            }
            for server, binary in BINARIES.items()
        }
        for platform in ("darwin-arm64", "linux-x86_64")
    }
    return {
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": REPOSITORY, "commit": COMMIT},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {
            "bootstrapApi": 1,
            "pluginRuntimeApi": 1,
            "transport": "stdio-newline-v1",
            "mcpProtocol": "2024-11-05",
        },
        "platforms": platforms,
    }


def release(archive_digest: str = "b" * 64) -> dict[str, object]:
    embedded = candidate()
    return {
        "schema": "codexy-runtime-release/v1",
        "state": "candidate-proven",
        "source": embedded["source"],
        "artifact": {
            "tag": "v1.3.0",
            "url": f"{REPOSITORY}/releases/download/v1.3.0/codexy-runtime-package.tar.gz",
            "sha256": archive_digest,
            "payloadManifestSha256": hashlib.sha256(encoded(embedded)).hexdigest(),
        },
        "compatibility": embedded["compatibility"],
        "platforms": embedded["platforms"],
    }


def write_candidate_archive(path: Path, *, mixed: bool = False) -> None:
    files = {
        "plugins/codexy-devtools/runtime-candidate.json": encoded(candidate()),
        "plugins/codexy-devtools/.codex-plugin/plugin.json": b'{"version":"1.3.0"}',
        **{
            f"plugins/codexy-devtools/runtime/codexy-mcp-{server}-{platform}.bin": binary
            for platform in ("darwin-arm64", "linux-x86_64")
            for server, binary in BINARIES.items()
        },
    }
    if mixed:
        files["plugins/codexy/.codex-plugin/plugin.json"] = b'{"version":"1.3.0"}'
    with tarfile.open(path, "w:gz") as archive:
        for name, contents in files.items():
            member = tarfile.TarInfo(name)
            member.size = len(contents)
            archive.addfile(member, io.BytesIO(contents))


class RuntimePackageRootTests(unittest.TestCase):
    def test_candidate_proven_archive_installs_from_devtools_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "candidate.tar.gz"
            write_candidate_archive(archive)
            selected = self.selected_release(root)
            installed = root / "cache/bin/codexy-mcp-lsp"
            install_package(self.config(archive, selected), root / "cache", installed)
            self.assertEqual(installed.read_bytes(), BINARIES["lsp"])
            self.assertEqual(
                (root / "cache/plugin.json").read_text(), '{"version":"1.3.0"}'
            )

    def test_selected_candidate_bootstraps_into_a_fresh_runtime_cache(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "candidate.tar.gz"
            write_candidate_archive(archive)
            manifest = root / ".codex-plugin/plugin.json"
            manifest.parent.mkdir()
            manifest.write_text('{"version":"1.3.0"}', encoding="utf-8")
            (root / "runtime-release.json").write_text(
                json.dumps(release(hashlib.sha256(archive.read_bytes()).hexdigest())),
                encoding="utf-8",
            )
            cache = root / "fresh-cache"
            with (
                mock.patch.dict(
                    os.environ,
                    {
                        "CODEXY_RUNTIME_CACHE_DIR": str(cache),
                        "CODEXY_RUNTIME_PLATFORM": "linux-x86_64",
                    },
                    clear=True,
                ),
                mock.patch.object(
                    package,
                    "_download",
                    side_effect=lambda _url, destination: shutil.copyfile(
                        archive, destination
                    ),
                ),
                mock.patch.object(runtime, "_execute", side_effect=Executed),
                self.assertRaises(Executed),
            ):
                runtime.run(runtime.Configuration.load("lsp", root, []))
            installed = next(cache.rglob("bin/codexy-mcp-lsp"))
            self.assertEqual(installed.read_bytes(), BINARIES["lsp"])

    def test_explicit_legacy_archive_still_installs_from_core_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "legacy.tar.gz"
            runtime_name = "codexy-mcp-lsp-linux-x86_64.bin"
            self.write_archive(
                archive,
                {
                    f"plugins/codexy/runtime/{runtime_name}": BINARIES["lsp"],
                    "plugins/codexy/.codex-plugin/plugin.json": b'{"version":"1.2.2"}',
                    "plugins/codexy-github/.codex-plugin/plugin.json": b'{"version":"1.2.2"}',
                },
            )
            installed = root / "cache/bin/codexy-mcp-lsp"
            config = SimpleNamespace(
                package_path=str(archive),
                package_url="",
                artifacts_api="",
                package_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                package_override=True,
                runtime_name=runtime_name,
                manifest=root / "plugin.json",
            )
            install_package(config, root / "cache", installed)
            self.assertEqual(installed.read_bytes(), BINARIES["lsp"])

    def test_candidate_proven_archive_rejects_mixed_core_and_devtools_roots(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "mixed.tar.gz"
            write_candidate_archive(archive, mixed=True)
            selected = self.selected_release(root)
            installed = root / "cache/bin/codexy-mcp-lsp"
            with self.assertRaisesRegex(RuntimeError, "mixed plugin roots"):
                install_package(
                    self.config(archive, selected), root / "cache", installed
                )
            self.assertFalse(installed.exists())

    @staticmethod
    def write_archive(path: Path, files: dict[str, bytes]) -> None:
        with tarfile.open(path, "w:gz") as archive:
            for name, contents in files.items():
                member = tarfile.TarInfo(name)
                member.size = len(contents)
                archive.addfile(member, io.BytesIO(contents))

    @staticmethod
    def selected_release(root: Path) -> contract.RuntimeRelease:
        (root / "runtime-release.json").write_text(
            json.dumps(release()), encoding="utf-8"
        )
        return contract.load(root)

    @staticmethod
    def config(archive: Path, selected: contract.RuntimeRelease) -> SimpleNamespace:
        return SimpleNamespace(
            package_path=str(archive),
            package_url="",
            artifacts_api="",
            package_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
            package_override=False,
            runtime_name="codexy-mcp-lsp-linux-x86_64.bin",
            manifest=archive,
            platform="linux-x86_64",
            release_contract=selected,
            source_identity=RuntimeSourceIdentity.create(
                explicit=None,
                package_sha256=selected.artifact.sha256,
                package_url=selected.artifact.url,
                release=selected,
            ),
        )


if __name__ == "__main__":
    unittest.main()
