"""Runtime package acquisition and archive handling cases."""

import io
import json
import tarfile
import tempfile
import urllib.request
import zipfile
import zlib
from pathlib import Path
from unittest import mock

from codexy_runtime_tools.package import (
    _GithubRedirectHandler,
    _artifact_package,
    _extract_zip,
    _safe_extract_tar,
    _safe_extract_zip,
    acquire_package,
)


class RuntimePackageBehaviorCases:
    def test_explicit_package_digest_must_match(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source.tar.gz"
            source.write_bytes(b"not the expected package")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                acquire_package(
                    path=str(source),
                    url="",
                    artifacts_api="",
                    expected_sha256="0" * 64,
                    work=root / "work",
                )

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
                    destination.write_text(
                        json.dumps(
                            {
                                "artifacts": [
                                    {"expired": False, "workflow_run": None},
                                    {"expired": False, "workflow_run": "main"},
                                    {
                                        "expired": False,
                                        "workflow_run": {
                                            "head_branch": "main",
                                            "head_repository_id": 1,
                                        },
                                        "archive_download_url": "https://api.github.com/fork.zip",
                                    },
                                    {
                                        "expired": False,
                                        "workflow_run": {
                                            "head_branch": "main",
                                            "head_repository_id": 1_269_350_143,
                                        },
                                        "archive_download_url": "https://api.github.com/valid.zip",
                                    },
                                ]
                            }
                        ),
                        encoding="utf-8",
                    )
                else:
                    with zipfile.ZipFile(destination, "w") as archive:
                        archive.writestr("codexy-marketplace-plugin.tar.gz", b"package")

            with (
                mock.patch(
                    "codexy_runtime_tools.package._github_token_for", return_value=""
                ),
                mock.patch(
                    "codexy_runtime_tools.package._download", side_effect=download
                ),
            ):
                self.assertEqual(
                    _artifact_package(api, root),
                    root / "artifact" / "codexy-marketplace-plugin.tar.gz",
                )

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

    def test_corrupt_deflate_has_runtime_diagnostic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "corrupt-deflate.zip"
            with zipfile.ZipFile(
                archive, "w", compression=zipfile.ZIP_DEFLATED
            ) as zipped:
                zipped.writestr("codexy-marketplace-plugin.tar.gz", b"payload-" * 1_000)
            with zipfile.ZipFile(archive) as zipped:
                info = zipped.infolist()[0]
                offset = (
                    info.header_offset
                    + 30
                    + len(info.filename.encode())
                    + len(info.extra)
                )
                compressed_size = info.compress_size
            contents = bytearray(archive.read_bytes())
            contents[offset : offset + compressed_size] = b"\0" * compressed_size
            archive.write_bytes(contents)
            with self.assertRaises(zlib.error):
                _extract_zip(archive, root / "raw")
            with self.assertRaisesRegex(ValueError, "invalid artifact archive"):
                _safe_extract_zip(archive, root / "safe")
