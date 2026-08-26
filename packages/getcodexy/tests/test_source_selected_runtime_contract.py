"""Source-checkout runtime selection is bound to the authenticated candidate."""

from __future__ import annotations

import hashlib
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools import contract

from runtime_contract_support import (
    BINARIES,
    encoded,
    source_candidate,
    source_selected,
)


class SourceSelectedRuntimeContractTests(unittest.TestCase):
    def test_source_selected_loads_without_a_tracked_candidate_and_verifies_archive(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "runtime.tar.gz"
            self.write_archive(archive)
            release = source_selected(hashlib.sha256(archive.read_bytes()).hexdigest())
            (root / "runtime-release.json").write_text(
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
