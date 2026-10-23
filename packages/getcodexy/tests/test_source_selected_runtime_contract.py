"""Source-checkout runtime selection is bound to the authenticated candidate."""

from __future__ import annotations

import hashlib
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path
from typing import cast

from codexy_runtime_tools import contract

from runtime_contract_support import (
    BINARIES,
    encoded,
    source_candidate,
    source_selected,
)


WATCHER_PLATFORM_KINDS = {
    "darwin-arm64": "mach-o",
    "linux-x86_64": "elf",
    "windows-x86_64": "pe",
}


def source_selected_with_legacy_watcher() -> tuple[
    dict[str, object], dict[str, object], dict[str, object]
]:
    """Bind legacy receipt metadata to the embedded candidate payload digest."""
    watcher: dict[str, object] = {
        "platforms": {
            platform: {
                "path": (
                    f"runtime/codexy-mcp-watcher-{platform}."
                    f"{'exe' if platform == 'windows-x86_64' else 'bin'}"
                ),
                "sha256": hashlib.sha256(platform.encode()).hexdigest(),
                "kind": kind,
            }
            for platform, kind in WATCHER_PLATFORM_KINDS.items()
        }
    }
    candidate = source_candidate()
    candidate_classes = cast(dict[str, object], candidate["classes"])
    candidate_classes["coreWatcherMcp"] = watcher
    receipt = source_selected()
    receipt_classes = cast(dict[str, object], receipt["classes"])
    receipt_classes["coreWatcherMcp"] = watcher
    artifact = cast(dict[str, object], receipt["artifact"])
    artifact["payloadManifestSha256"] = hashlib.sha256(encoded(candidate)).hexdigest()
    return receipt, watcher, candidate


class SourceSelectedRuntimeContractTests(unittest.TestCase):
    def test_source_selected_loads_without_a_tracked_candidate_and_verifies_archive(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "runtime.tar.gz"
            self.write_archive(archive)
            release = source_selected(hashlib.sha256(archive.read_bytes()).hexdigest())
            _ = (root / "runtime-release.json").write_text(
                json.dumps(release), encoding="utf-8"
            )

            parsed = contract.load(root)

            self.assertEqual(parsed.state, "source-selected")
            self.assertEqual(parsed.source.tree, "c" * 40)
            self.assertEqual(parsed.package_plugin_root(), "codexy-devtools")
            self.assertTrue(parsed.advertises(platform="darwin-arm64"))
            self.assertFalse(parsed.advertises(platform="windows-x86_64"))
            self.assertTrue(parsed.verify_archive(archive, platform="linux-x86_64"))
            marker = parsed.marker(
                platform="linux-x86_64",
                server="lsp",
                binary_sha256=hashlib.sha256(BINARIES["lsp"]).hexdigest(),
            )
            self.assertEqual(marker["identity"]["source"]["tree"], "c" * 40)
            self.assertIn("provenance", marker["identity"])
            self.assertIn("classes", marker["identity"])

    def test_source_selected_receipt_preserves_optional_legacy_watcher_class(
        self,
    ) -> None:
        receipt, watcher, _candidate = source_selected_with_legacy_watcher()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _ = (root / "runtime-release.json").write_text(
                json.dumps(receipt), encoding="utf-8"
            )

            parsed = contract.load(root)

        classes = parsed.classes
        self.assertIsNotNone(classes)
        assert classes is not None
        self.assertEqual(cast(object, classes["coreWatcherMcp"]), watcher)

    def test_source_selected_receipt_rejects_noncanonical_legacy_watcher_path(
        self,
    ) -> None:
        receipt, watcher, candidate = source_selected_with_legacy_watcher()
        platforms = cast(dict[str, object], watcher["platforms"])
        windows = cast(dict[str, object], platforms["windows-x86_64"])
        windows["path"] = "runtime/codexy-mcp-watcher-windows-x86_64.bin"
        artifact = cast(dict[str, object], receipt["artifact"])
        artifact["payloadManifestSha256"] = hashlib.sha256(
            encoded(candidate)
        ).hexdigest()

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "runtime-release.json").write_text(
                json.dumps(receipt), encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "path is not canonical"):
                _ = contract.load(root)

    @staticmethod
    def write_archive(path: Path) -> None:
        embedded = source_candidate()
        files = {
            "plugins/codexy-devtools/runtime-candidate.json": encoded(embedded),
            "plugins/codexy-devtools/.codex-plugin/plugin.json": b'{"version":"1.5.0"}',
            **{
                f"plugins/codexy-devtools/runtime/codexy-mcp-{server}-{platform}.{'exe' if platform == 'windows-x86_64' else 'bin'}": binary
                for platform in (
                    "darwin-arm64",
                    "linux-x86_64",
                    "windows-x86_64",
                )
                for server, binary in BINARIES.items()
            },
        }
        with tarfile.open(path, "w:gz") as archive:
            for name, contents in files.items():
                member = tarfile.TarInfo(name)
                member.size = len(contents)
                archive.addfile(member, io.BytesIO(contents))


if __name__ == "__main__":
    unittest.main()
