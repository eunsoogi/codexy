"""RED contract for immutable public runtime selection (Issue #477)."""

import hashlib
import importlib
import io
import json
import os
import tarfile
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from unittest import mock


TAG = "runtime-candidate-2026-07-22.1"
COMMIT = "a" * 40
ARCHIVE_DIGEST = "b" * 64
URL = f"https://github.com/eunsoogi/codexy/releases/download/{TAG}/codexy-marketplace-plugin.tar.gz"
BINARIES = {"lsp": b"lsp binary", "codegraph": b"codegraph binary"}


def contract_module():
    """Import per test so the future module fails in test execution, not collection."""
    return importlib.import_module("codexy_runtime_tools.contract")


def encoded(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def candidate() -> dict[str, object]:
    platforms = {
        platform: {name: {"path": f"runtime/codexy-mcp-{name}-{platform}.bin", "sha256": hashlib.sha256(data).hexdigest()}
            for name, data in BINARIES.items()}
        for platform in ("darwin-arm64", "linux-x86_64")
    }
    return {
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": COMMIT},
        "artifact": {"tag": TAG},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": platforms,
    }


def release() -> dict[str, object]:
    embedded = candidate()
    return {
        "schema": "codexy-runtime-release/v1",
        "state": "candidate-proven",
        "source": embedded["source"],
        "artifact": {"tag": TAG, "url": URL, "sha256": ARCHIVE_DIGEST, "payloadManifestSha256": hashlib.sha256(encoded(embedded)).hexdigest()},
        "compatibility": embedded["compatibility"],
        "platforms": embedded["platforms"],
    }


def legacy() -> dict[str, object]:
    value = release()
    value["state"] = "legacy-public"
    value["platforms"] = {
        platform: {server: {"sha256": binary["sha256"]} for server, binary in inventory.items()}
        for platform, inventory in candidate()["platforms"].items()  # type: ignore[union-attr]
    }
    return value


class RuntimeContractTests(unittest.TestCase):
    def load(self, contents: dict[str, object], *, plugin_version: str = "9.9.9"):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        manifest = root / ".codex-plugin" / "plugin.json"
        manifest.parent.mkdir()
        manifest.write_text(json.dumps({"name": "codexy", "version": plugin_version}), encoding="utf-8")
        (root / "runtime-release.json").write_text(json.dumps(contents), encoding="utf-8")
        return root, contract_module().load(root)

    def test_v1_schema_uses_standalone_contract_not_plugin_version(self) -> None:
        root, parsed = self.load(release(), plugin_version="99.99.99")
        self.assertEqual(parsed.artifact.tag, TAG)
        self.assertEqual(parsed.compatibility.bootstrap_api, 1)
        bad = release()
        bad["schema"] = "codexy-runtime-release/v2"
        with self.assertRaises(ValueError):
            self.load(bad)
        bad = release()
        bad["unexpected"] = True
        with self.assertRaises(ValueError):
            self.load(bad)
        (root / "runtime-release.json").write_text('{"schema":"x","schema":"y"}', encoding="utf-8")
        with self.assertRaises(ValueError):
            contract_module().load(root)

    def test_requires_canonical_artifact_source_and_lowercase_digests(self) -> None:
        _, parsed = self.load(release())
        self.assertEqual(parsed.artifact.url, URL)
        self.assertEqual(parsed.source.commit, COMMIT)
        for field, value in (("url", "https://example.test/x"), ("url", URL.replace(TAG, "other")), ("sha256", "B" * 64), ("payloadManifestSha256", "z" * 64)):
            bad = release()
            bad["artifact"][field] = value  # type: ignore[index]
            with self.assertRaises(ValueError):
                self.load(bad)
        bad = release()
        bad["source"]["commit"] = "main"  # type: ignore[index]
        with self.assertRaises(ValueError):
            self.load(bad)

    def test_rejects_unknown_inventory_and_protocols(self) -> None:
        for change in (
            lambda value: value["platforms"].update({"windows-x86_64": {}}),
            lambda value: value["platforms"]["linux-x86_64"].update({"other": {}}),
            lambda value: value["compatibility"].update({"transport": "stdio"}),
            lambda value: value["compatibility"].update({"mcpProtocol": "wrong"}),
            lambda value: value["platforms"]["linux-x86_64"]["lsp"].update({"path": "plugins/codexy/runtime/../bad"}),
        ):
            bad = release()
            change(bad)
            with self.assertRaises(ValueError):
                self.load(bad)

    def test_compatibility_and_windows_advertising_fail_closed(self) -> None:
        _, parsed = self.load(release())
        self.assertTrue(parsed.supports(server="lsp", platform="linux-x86_64", bootstrap_api=1, plugin_runtime_api=1, transport="stdio-newline-v1", mcp_protocol="2024-11-05"))
        self.assertFalse(parsed.supports(server="lsp", platform="windows-x86_64", bootstrap_api=1, plugin_runtime_api=1, transport="stdio-newline-v1", mcp_protocol="2024-11-05"))
        self.assertFalse(parsed.supports(server="lsp", platform="linux-x86_64", bootstrap_api=0, plugin_runtime_api=1, transport="stdio-newline-v1", mcp_protocol="2024-11-05"))
        self.assertFalse(parsed.advertises(platform="windows-x86_64"))

    def test_cache_uses_runtime_identity_not_plugin_version_and_rolls_back(self) -> None:
        _, prior = self.load(release(), plugin_version="1.2.2")
        _, future = self.load(release(), plugin_version="9.9.9")
        changed = release()
        changed["artifact"]["tag"] = "runtime-candidate-2026-07-23.1"  # type: ignore[index]
        changed["artifact"]["url"] = URL.replace(TAG, "runtime-candidate-2026-07-23.1")  # type: ignore[index]
        _, advanced = self.load(changed)
        self.assertEqual(prior.cache_key(platform="linux-x86_64", server="lsp"), future.cache_key(platform="linux-x86_64", server="lsp"))
        self.assertNotEqual(prior.cache_key(platform="linux-x86_64", server="lsp"), advanced.cache_key(platform="linux-x86_64", server="lsp"))
        self.assertNotEqual(prior.cache_key(platform="darwin-arm64", server="lsp"), prior.cache_key(platform="linux-x86_64", server="lsp"))
        self.assertNotEqual(prior.cache_key(platform="linux-x86_64", server="lsp"), prior.cache_key(platform="linux-x86_64", server="codegraph"))
        changed = release()
        changed["artifact"]["sha256"] = "c" * 64  # type: ignore[index]
        _, digest_changed = self.load(changed)
        protocol_changed = replace(prior, compatibility=replace(prior.compatibility, mcp_protocol="2025-01-01"))
        self.assertNotEqual(prior.cache_key(platform="linux-x86_64", server="lsp"), digest_changed.cache_key(platform="linux-x86_64", server="lsp"))
        self.assertNotEqual(prior.cache_key(platform="linux-x86_64", server="lsp"), protocol_changed.cache_key(platform="linux-x86_64", server="lsp"))

    def test_explicit_override_cannot_poison_selected_release_cache(self) -> None:
        root, _ = self.load(legacy())
        runtime = importlib.import_module("codexy_runtime_tools.runtime")
        cache = root / "cache"
        override = root / "override.tar.gz"
        override.write_bytes(b"override archive")
        installed_roots: list[Path] = []

        def install_override(_config, install_root: Path, installed: Path) -> None:
            installed.parent.mkdir(parents=True)
            installed.write_bytes(b"override controlled runtime")
            installed.chmod(0o755)
            installed_roots.append(install_root)

        environment = {
            "CODEXY_RUNTIME_CACHE_DIR": str(cache),
            "CODEXY_RUNTIME_PACKAGE_PATH": str(override),
            "CODEXY_RUNTIME_PACKAGE_SHA256": hashlib.sha256(override.read_bytes()).hexdigest(),
        }
        with mock.patch.dict(os.environ, environment, clear=True), mock.patch.object(runtime, "install_package", side_effect=install_override), mock.patch.object(runtime, "_execute", side_effect=SystemExit(0)), self.assertRaises(SystemExit):
            runtime.run(runtime.Configuration.load("lsp", root, []))
        self.assertEqual(len(installed_roots), 1)

        with mock.patch.dict(os.environ, {"CODEXY_RUNTIME_CACHE_DIR": str(cache), "UV_OFFLINE": "1"}, clear=True), mock.patch.object(runtime, "_execute") as execute, self.assertRaises(SystemExit) as failure:
            runtime.run(runtime.Configuration.load("lsp", root, []))
        self.assertEqual(failure.exception.code, 127)
        execute.assert_not_called()

    def test_marker_rejects_stale_identity_and_binary_digest(self) -> None:
        _, parsed = self.load(release())
        binary = BINARIES["lsp"]
        marker = parsed.marker(platform="linux-x86_64", server="lsp", binary_sha256=hashlib.sha256(binary).hexdigest())
        self.assertTrue(parsed.valid_marker(marker, platform="linux-x86_64", server="lsp", binary=binary))
        stale = {**marker, "identity": {**marker["identity"], "artifact": {**marker["identity"]["artifact"], "tag": "stale"}}}
        self.assertFalse(parsed.valid_marker(stale, platform="linux-x86_64", server="lsp", binary=binary))
        self.assertFalse(parsed.valid_marker(marker, platform="linux-x86_64", server="lsp", binary=b"tampered"))

    def test_archive_requires_candidate_digest_identity_and_binary_inventory(self) -> None:
        _, parsed = self.load(release())
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "runtime.tar.gz"
            self.archive(archive, candidate())
            self.assertTrue(parsed.verify_archive(archive, platform="linux-x86_64"))
            bad = candidate()
            bad["artifact"]["tag"] = "other"  # type: ignore[index]
            self.archive(archive, bad)
            with self.assertRaises(ValueError):
                parsed.verify_archive(archive, platform="linux-x86_64")

    def test_legacy_contract_selects_public_release_after_plugin_version_changes(self) -> None:
        root, parsed = self.load(legacy(), plugin_version="99.99.99")
        runtime = importlib.import_module("codexy_runtime_tools.runtime")
        with mock.patch.dict(os.environ, {}, clear=True):
            configuration = runtime.Configuration.load("lsp", root, [])
        self.assertEqual(configuration.package_url, URL)
        self.assertEqual(configuration.package_sha256, ARCHIVE_DIGEST)
        self.assertEqual(configuration.release_contract, parsed)
        self.assertTrue(parsed.verify_archive(root / "missing.tar.gz", platform="linux-x86_64"))

    def test_runtime_boundary_never_invokes_cargo_without_explicit_exact_fallback(self) -> None:
        root, _ = self.load(release())
        runtime = importlib.import_module("codexy_runtime_tools.runtime")
        environment = {"CODEXY_RUNTIME_CACHE_DIR": str(root / "cache")}
        with mock.patch.dict(os.environ, environment, clear=True), mock.patch.object(runtime, "install_package", side_effect=RuntimeError("missing public artifact")), mock.patch.object(runtime, "install_git") as cargo, self.assertRaises(SystemExit):
            runtime.run(runtime.Configuration.load("lsp", root, []))
        cargo.assert_not_called()
        with mock.patch.dict(os.environ, {**environment, "CODEXY_RUNTIME_GIT_FALLBACK": "1"}, clear=True), mock.patch.object(runtime, "install_package", side_effect=RuntimeError("missing public artifact")), mock.patch.object(runtime, "install_git", side_effect=RuntimeError("cargo failed")) as cargo, self.assertRaises(SystemExit):
            configuration = runtime.Configuration.load("lsp", root, [])
            self.assertEqual(configuration.git_ref, COMMIT)
            runtime.run(configuration)
        cargo.assert_called_once()

    @staticmethod
    def archive(path: Path, embedded: dict[str, object]) -> None:
        with tarfile.open(path, "w:gz") as packaged:
            files = {
                "plugins/codexy/runtime-candidate.json": encoded(embedded),
                "plugins/codexy/.codex-plugin/plugin.json": b'{"version":"1.2.2"}',
                **{f"plugins/codexy/runtime/codexy-mcp-{server}-linux-x86_64.bin": data for server, data in BINARIES.items()},
            }
            for name, data in files.items():
                info = tarfile.TarInfo(name)
                info.size = len(data)
                packaged.addfile(info, io.BytesIO(data))


if __name__ == "__main__":
    unittest.main()
