"""Contract for version-only public runtime selection."""

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

from runtime_contract_support import (
    ARCHIVE_DIGEST,
    BINARIES,
    COMMIT,
    LEGACY_URL,
    TAG,
    URL,
    candidate,
    contract_module,
    encoded,
    legacy,
    release,
)
from runtime_contract_runtime_cases import RuntimeContractRuntimeCases


class RuntimeContractTests(RuntimeContractRuntimeCases, unittest.TestCase):
    def load(self, contents: dict[str, object], *, plugin_version: str = "9.9.9"):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        manifest = root / ".codex-plugin" / "plugin.json"
        manifest.parent.mkdir()
        manifest.write_text(
            json.dumps({"name": "codexy", "version": plugin_version}), encoding="utf-8"
        )
        (root / "runtime-release.json").write_text(
            json.dumps(contents), encoding="utf-8"
        )
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
        (root / "runtime-release.json").write_text(
            '{"schema":"x","schema":"y"}', encoding="utf-8"
        )
        with self.assertRaises(ValueError):
            contract_module().load(root)

    def test_requires_canonical_artifact_source_and_lowercase_digests(self) -> None:
        _, parsed = self.load(release())
        self.assertEqual(parsed.artifact.url, URL)
        self.assertEqual(parsed.source.commit, COMMIT)
        for field, value in (
            ("url", "https://example.test/x"),
            ("url", URL.replace(TAG, "other")),
            ("sha256", "B" * 64),
            ("payloadManifestSha256", "z" * 64),
        ):
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
            lambda value: value["platforms"]["linux-x86_64"]["lsp"].update(
                {"path": "plugins/codexy-devtools/runtime/../bad"}
            ),
        ):
            bad = release()
            change(bad)
            with self.assertRaises(ValueError):
                self.load(bad)

    def test_compatibility_and_legacy_windows_advertising_fail_closed(self) -> None:
        _, parsed = self.load(release())
        self.assertTrue(
            parsed.supports(
                server="lsp",
                platform="linux-x86_64",
                bootstrap_api=1,
                plugin_runtime_api=1,
                transport="stdio-newline-v1",
                mcp_protocol="2024-11-05",
            )
        )
        self.assertTrue(
            parsed.supports(
                server="lsp",
                platform="windows-x86_64",
                bootstrap_api=1,
                plugin_runtime_api=1,
                transport="stdio-newline-v1",
                mcp_protocol="2024-11-05",
            )
        )
        self.assertFalse(
            parsed.supports(
                server="lsp",
                platform="linux-x86_64",
                bootstrap_api=0,
                plugin_runtime_api=1,
                transport="stdio-newline-v1",
                mcp_protocol="2024-11-05",
            )
        )
        self.assertTrue(parsed.advertises(platform="windows-x86_64"))
        self.assertFalse(self.load(legacy())[1].advertises(platform="windows-x86_64"))

    def test_cache_uses_runtime_identity_not_plugin_version_and_rolls_back(
        self,
    ) -> None:
        _, prior = self.load(release(), plugin_version="1.2.2")
        _, future = self.load(release(), plugin_version="9.9.9")
        changed = release()
        changed["artifact"]["tag"] = "v1.3.1"  # type: ignore[index]
        changed["artifact"]["url"] = URL.replace(TAG, "v1.3.1")  # type: ignore[index]
        _, advanced = self.load(changed)
        self.assertEqual(
            prior.cache_key(platform="linux-x86_64", server="lsp"),
            future.cache_key(platform="linux-x86_64", server="lsp"),
        )
        self.assertNotEqual(
            prior.cache_key(platform="linux-x86_64", server="lsp"),
            advanced.cache_key(platform="linux-x86_64", server="lsp"),
        )
        self.assertNotEqual(
            prior.cache_key(platform="darwin-arm64", server="lsp"),
            prior.cache_key(platform="linux-x86_64", server="lsp"),
        )
        self.assertNotEqual(
            prior.cache_key(platform="linux-x86_64", server="lsp"),
            prior.cache_key(platform="linux-x86_64", server="codegraph"),
        )
        changed = release()
        changed["artifact"]["sha256"] = "c" * 64  # type: ignore[index]
        _, digest_changed = self.load(changed)
        protocol_changed = replace(
            prior, compatibility=replace(prior.compatibility, mcp_protocol="2025-01-01")
        )
        self.assertNotEqual(
            prior.cache_key(platform="linux-x86_64", server="lsp"),
            digest_changed.cache_key(platform="linux-x86_64", server="lsp"),
        )
        self.assertNotEqual(
            prior.cache_key(platform="linux-x86_64", server="lsp"),
            protocol_changed.cache_key(platform="linux-x86_64", server="lsp"),
        )

    def test_marker_rejects_stale_identity_and_binary_digest(self) -> None:
        _, parsed = self.load(release())
        binary = BINARIES["lsp"]
        marker = parsed.marker(
            platform="linux-x86_64",
            server="lsp",
            binary_sha256=hashlib.sha256(binary).hexdigest(),
        )
        self.assertTrue(
            parsed.valid_marker(
                marker, platform="linux-x86_64", server="lsp", binary=binary
            )
        )
        stale = {
            **marker,
            "identity": {
                **marker["identity"],
                "artifact": {**marker["identity"]["artifact"], "tag": "stale"},
            },
        }
        self.assertFalse(
            parsed.valid_marker(
                stale, platform="linux-x86_64", server="lsp", binary=binary
            )
        )
        self.assertFalse(
            parsed.valid_marker(
                marker, platform="linux-x86_64", server="lsp", binary=b"tampered"
            )
        )

    def test_archive_requires_candidate_identity_and_binary_inventory(self) -> None:
        _, parsed = self.load(release())
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "runtime.tar.gz"
            self.archive(archive, candidate())
            self.assertTrue(parsed.verify_archive(archive, platform="linux-x86_64"))
            bad = candidate()
            bad["artifact"]["stagingRunId"] = 99  # type: ignore[index]
            self.archive(archive, bad)
            with self.assertRaises(ValueError):
                parsed.verify_archive(archive, platform="linux-x86_64")

    def test_legacy_contract_selects_public_release_after_plugin_version_changes(
        self,
    ) -> None:
        root, parsed = self.load(legacy(), plugin_version="99.99.99")
        runtime = importlib.import_module("codexy_runtime_tools.runtime")
        with mock.patch.dict(os.environ, {}, clear=True):
            configuration = runtime.Configuration.load("lsp", root, [])
        self.assertEqual(configuration.package_url, LEGACY_URL)
        self.assertEqual(configuration.package_sha256, ARCHIVE_DIGEST)
        self.assertEqual(configuration.release_contract, parsed)
        self.assertTrue(
            parsed.verify_archive(root / "missing.tar.gz", platform="linux-x86_64")
        )

    @staticmethod
    def archive(path: Path, embedded: dict[str, object]) -> None:
        with tarfile.open(path, "w:gz") as packaged:
            files = {
                "plugins/codexy-devtools/runtime-candidate.json": encoded(embedded),
                "plugins/codexy-devtools/.codex-plugin/plugin.json": b'{"version":"1.2.2"}',
                **{
                    f"plugins/codexy-devtools/runtime/codexy-mcp-{server}-{platform}.{'exe' if platform == 'windows-x86_64' else 'bin'}": data
                    for platform in ("darwin-arm64", "linux-x86_64", "windows-x86_64")
                    for server, data in BINARIES.items()
                },
            }
            for name, data in files.items():
                info = tarfile.TarInfo(name)
                info.size = len(data)
                packaged.addfile(info, io.BytesIO(data))


if __name__ == "__main__":
    unittest.main()
