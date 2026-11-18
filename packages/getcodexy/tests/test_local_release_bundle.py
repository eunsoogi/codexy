"""Synthetic public asset/attestation boundary tests, never contacting GitHub."""

from __future__ import annotations

import hashlib
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools import local_release_bundle as subject

PLUGINS = ("codexy", "codexy-github", "codexy-devtools")


class PublicFixture:
    def __init__(self):
        self.files = {}
        records, entries = [], []
        for plugin in PLUGINS:
            root = f"plugins/{plugin}"
            data = json.dumps(
                {
                    "name": plugin,
                    "version": "2.3.0",
                    "repository": "https://github.com/eunsoogi/codexy",
                }
            ).encode()
            self.files[f"{root}/.codex-plugin/plugin.json"] = data
            records.append(
                {
                    "plugin": plugin,
                    "version": "2.3.0",
                    "packageRoot": root,
                    "manifestSha256": hashlib.sha256(data).hexdigest(),
                }
            )
            entries.append(
                {
                    "name": plugin,
                    "version": "2.3.0",
                    "source": {"source": "local", "path": f"./{root}"},
                }
            )
        self.files[".agents/plugins/marketplace.json"] = json.dumps(
            {"name": "codexy", "plugins": entries}
        ).encode()
        self.receipt = {
            "schema": "codexy-runtime-release-receipt/v2",
            "source": {"activationCommit": "a" * 40},
            "release": {"tag": "v2.3.0"},
            "components": records,
        }
        self.release = {"tag_name": "v2.3.0", "draft": False, "prerelease": False}
        self.commands = []
        self.corrupt_download = False
        self.attestation_failure = False
        self.wrong_subject = False
        self.pack()

    def pack(self):
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
            for name, data in self.files.items():
                member = tarfile.TarInfo(name)
                member.size = len(data)
                archive.addfile(member, io.BytesIO(data))
        bundle = buffer.getvalue()
        self.receipt["bundleArtifact"] = {
            "name": subject.BUNDLE,
            "sha256": hashlib.sha256(bundle).hexdigest(),
        }
        self.assets = {
            subject.BUNDLE: bundle,
            subject.RECEIPT: json.dumps(self.receipt).encode(),
        }
        self.release["assets"] = [
            {
                "name": name,
                "digest": f"sha256:{hashlib.sha256(data).hexdigest()}",
                "state": "uploaded",
                "browser_download_url": f"https://github.com/eunsoogi/codexy/releases/download/v2.3.0/{name}",
            }
            for name, data in self.assets.items()
        ]

    def command(self, args):
        self.commands.append(args)
        if args[1] == "api":
            return self.release
        if self.attestation_failure:
            raise ValueError("verification failed")
        path = Path(args[3])
        name = "foreign" if self.wrong_subject else path.name
        return [
            {
                "verificationResult": {
                    "statement": {
                        "subject": [
                            {
                                "name": name,
                                "digest": {
                                    "sha256": hashlib.sha256(
                                        path.read_bytes()
                                    ).hexdigest()
                                },
                            }
                        ]
                    }
                }
            }
        ]

    def download(self, url, destination):
        destination.write_bytes(
            self.assets[destination.name] + (b"bad" if self.corrupt_download else b"")
        )

    def acquire(self, home):
        with (
            patch.object(subject, "_command", self.command),
            patch.object(subject, "_download", self.download),
        ):
            return subject.acquire_target(home, "2.3.0", PLUGINS)


class LocalReleaseBundleTests(unittest.TestCase):
    def test_independently_verified_new_root_and_exact_attestation_policy(self):
        fixture = PublicFixture()
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary).resolve()
            old = home / "old"
            old.write_bytes(b"immutable")
            identity = fixture.acquire(home)
            self.assertEqual(identity.source_type, "local")
            self.assertEqual(identity.source_value, str(identity.root))
            self.assertTrue(
                identity.root.is_relative_to(home / "getcodexy/marketplaces")
            )
            self.assertEqual(old.read_bytes(), b"immutable")
            second = fixture.acquire(home)
            self.assertNotEqual(identity.root, second.root)
        for command in fixture.commands:
            if command[1] != "attestation":
                continue
            self.assertIn("--deny-self-hosted-runners", command)
            self.assertEqual(command[command.index("--source-digest") + 1], "a" * 40)
            self.assertEqual(
                command[command.index("--source-ref") + 1], "refs/heads/main"
            )
            self.assertEqual(
                command[command.index("--signer-workflow") + 1],
                "eunsoogi/codexy/.github/workflows/publish-version-release.yml",
            )

    def test_invalid_assets_fail_closed_and_remove_partial_target(self):
        for failure in (
            "digest",
            "signature",
            "subject",
            "ambiguous",
            "foreign",
            "draft",
            "prerelease",
            "receipt-tag",
            "mixed-version",
            "duplicate-plugin",
            "unsafe-path",
        ):
            with (
                self.subTest(failure=failure),
                tempfile.TemporaryDirectory() as temporary,
            ):
                fixture = PublicFixture()
                if failure == "digest":
                    fixture.corrupt_download = True
                elif failure == "signature":
                    fixture.attestation_failure = True
                elif failure == "subject":
                    fixture.wrong_subject = True
                elif failure == "ambiguous":
                    fixture.release["assets"].append(fixture.release["assets"][0])
                elif failure == "foreign":
                    fixture.release["assets"][0]["browser_download_url"] = (
                        "https://example.invalid/bundle"
                    )
                elif failure in {"draft", "prerelease"}:
                    fixture.release[failure] = True
                elif failure == "receipt-tag":
                    fixture.receipt["release"]["tag"] = "v2.2.4"
                    fixture.pack()
                elif failure == "mixed-version":
                    fixture.receipt["components"][0]["version"] = "2.2.4"
                    fixture.pack()
                elif failure == "duplicate-plugin":
                    path = ".agents/plugins/marketplace.json"
                    data = json.loads(fixture.files[path])
                    data["plugins"][1] = data["plugins"][0]
                    fixture.files[path] = json.dumps(data).encode()
                    fixture.pack()
                else:
                    fixture.files["../escape"] = b"bad"
                    fixture.pack()
                home = Path(temporary).resolve()
                with self.assertRaises(ValueError):
                    fixture.acquire(home)
                self.assertEqual(list((home / "getcodexy/marketplaces").iterdir()), [])
                self.assertFalse((home / "escape").exists())

    def test_invalid_version_rejected_before_io(self):
        with patch.object(subject, "_command") as command:
            with self.assertRaises(ValueError):
                subject.acquire_target(Path("/missing"), "../../invalid", PLUGINS)
            command.assert_not_called()
