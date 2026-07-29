"""Negative coverage for the removed public runtime-candidate Release model."""

import hashlib
import importlib
import json
import tempfile
import unittest
from pathlib import Path


TAG = "v1.3.0"
COMMIT = "a" * 40
ARCHIVE_DIGEST = "b" * 64
REPOSITORY = "https://github.com/eunsoogi/codexy"


class CandidateReleaseContractTests(unittest.TestCase):
    def load(self, contents: dict[str, object]) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        manifest = root / ".codex-plugin" / "plugin.json"
        manifest.parent.mkdir()
        manifest.write_text(
            json.dumps({"name": "codexy", "version": "1.3.0"}),
            encoding="utf-8",
        )
        (root / "runtime-release.json").write_text(
            json.dumps(contents),
            encoding="utf-8",
        )
        importlib.import_module("codexy_runtime_tools.contract").load(root)

    def test_rejects_candidate_release_tags_in_every_runtime_state(self) -> None:
        for state in ("legacy-public", "candidate-proven"):
            former = release()
            former["state"] = state
            if state == "legacy-public":
                former["platforms"] = legacy_platforms()
            former["artifact"]["tag"] = "runtime-candidate-1.3.0"
            former["artifact"]["url"] = (
                f"{REPOSITORY}/releases/download/"
                "runtime-candidate-1.3.0/codexy-marketplace-plugin.tar.gz"
            )
            with self.assertRaises(ValueError):
                self.load(former)

    def test_candidate_proven_runtime_uses_dedicated_immutable_asset(self) -> None:
        former = release()
        with self.assertRaises(ValueError):
            self.load(former)


def release() -> dict[str, object]:
    platforms = {
        platform: {
            server: {
                "path": f"runtime/codexy-mcp-{server}-{platform}.bin",
                "sha256": hashlib.sha256(server.encode()).hexdigest(),
            }
            for server in ("lsp", "codegraph")
        }
        for platform in ("darwin-arm64", "linux-x86_64")
    }
    return {
        "schema": "codexy-runtime-release/v1",
        "state": "candidate-proven",
        "source": {"repository": REPOSITORY, "commit": COMMIT},
        "artifact": {
            "tag": TAG,
            "url": f"{REPOSITORY}/releases/download/{TAG}/codexy-marketplace-plugin.tar.gz",
            "sha256": ARCHIVE_DIGEST,
            "payloadManifestSha256": "c" * 64,
        },
        "compatibility": {
            "bootstrapApi": 1,
            "pluginRuntimeApi": 1,
            "transport": "stdio-newline-v1",
            "mcpProtocol": "2024-11-05",
        },
        "platforms": platforms,
    }


def legacy_platforms() -> dict[str, object]:
    return {
        platform: {
            server: {"sha256": hashlib.sha256(server.encode()).hexdigest()}
            for server in ("lsp", "codegraph")
        }
        for platform in ("darwin-arm64", "linux-x86_64")
    }


if __name__ == "__main__":
    unittest.main()
