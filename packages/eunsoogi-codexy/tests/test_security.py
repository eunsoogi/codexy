import io
import json
import tarfile
import tempfile
import unittest
import urllib.request
import zipfile
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import package
from codexy_runtime_tools.package import (
    _GithubRedirectHandler,
    _artifact_package,
    _github_token_for,
    _safe_extract_tar,
    _safe_extract_zip,
    acquire_package,
)


class ArchiveSecurityTests(unittest.TestCase):
    def test_artifact_listing_skips_non_object_workflow_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifacts_api = "https://api.github.com/repos/eunsoogi/codexy/actions/artifacts"

            def download(url: str, destination: Path, token: str = "") -> None:
                if url == artifacts_api:
                    destination.write_text(
                        json.dumps(
                            {
                                "artifacts": [
                                    {"expired": False, "workflow_run": None},
                                    {"expired": False, "workflow_run": "main"},
                                    {"expired": False, "workflow_run": []},
                                    {"expired": False, "workflow_run": 1},
                                    {
                                        "expired": False,
                                        "workflow_run": {"head_branch": "main", "head_repository_id": 1},
                                        "archive_download_url": "https://api.github.com/fork-artifact.zip",
                                    },
                                    {
                                        "expired": False,
                                        "workflow_run": {"head_branch": "main", "head_repository_id": 1_269_350_143},
                                        "archive_download_url": "https://api.github.com/artifact.zip",
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
                mock.patch("codexy_runtime_tools.package._github_token_for", return_value=""),
                mock.patch("codexy_runtime_tools.package._download", side_effect=download),
            ):
                self.assertEqual(_artifact_package(artifacts_api, root), root / "artifact" / "codexy-marketplace-plugin.tar.gz")

    def test_github_environment_token_takes_precedence_over_cli_auth(self) -> None:
        with mock.patch.dict(
            "os.environ", {"GH_TOKEN": "environment-token", "GITHUB_TOKEN": "other"}, clear=True
        ), mock.patch.object(package.subprocess, "run") as run:
            self.assertEqual(
                _github_token_for("https://api.github.com/repos/eunsoogi/codexy/actions/artifacts"),
                "environment-token",
            )
        run.assert_not_called()

    def test_github_cli_auth_token_is_used_when_environment_is_empty(self) -> None:
        with mock.patch.dict("os.environ", {}, clear=True), mock.patch.object(
            package.subprocess,
            "run",
            return_value=package.subprocess.CompletedProcess(["gh"], 0, "cli-token\n", ""),
        ) as run:
            self.assertEqual(
                _github_token_for("https://api.github.com/repos/eunsoogi/codexy/actions/artifacts"),
                "cli-token",
            )
        run.assert_called_once_with(
            ["gh", "auth", "token"],
            check=True,
            stdout=package.subprocess.PIPE,
            stderr=package.subprocess.DEVNULL,
            text=True,
        )

    def test_missing_or_failed_github_cli_auth_returns_no_token(self) -> None:
        for error in (
            FileNotFoundError(),
            package.subprocess.CalledProcessError(1, ["gh", "auth", "token"]),
        ):
            with self.subTest(error=type(error).__name__), mock.patch.dict(
                "os.environ", {}, clear=True
            ), mock.patch.object(package.subprocess, "run", side_effect=error):
                self.assertEqual(
                    _github_token_for("https://api.github.com/repos/eunsoogi/codexy/actions/artifacts"),
                    "",
                )

    def test_untrusted_artifact_host_never_uses_github_cli_auth(self) -> None:
        with mock.patch.dict("os.environ", {}, clear=True), mock.patch.object(
            package.subprocess, "run"
        ) as run:
            self.assertEqual(_github_token_for("https://objects.example.test/artifact.zip"), "")
        run.assert_not_called()

    def test_cross_host_redirect_drops_github_authorization(self) -> None:
        request = urllib.request.Request(
            "https://api.github.com/repos/eunsoogi/codexy/actions/artifacts/1/zip",
            headers={"Authorization": "Bearer secret"},
        )
        redirected = _GithubRedirectHandler().redirect_request(
            request,
            None,
            302,
            "Found",
            {},
            "https://objects.example.test/artifact.zip",
        )

        self.assertIsNotNone(redirected)
        self.assertIsNone(redirected.get_header("Authorization"))

    def test_malformed_tar_is_translated_to_package_diagnostic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "malformed.tar.gz"
            archive.write_bytes(b"not a gzip archive")
            with self.assertRaisesRegex(ValueError, "invalid runtime package archive"):
                _safe_extract_tar(archive, root / "extract")

    def test_truncated_gzip_tar_is_translated_to_package_diagnostic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "truncated.tar.gz"
            with tarfile.open(archive, "w:gz") as package:
                member = tarfile.TarInfo("plugins/codexy/plugin.json")
                member.size = 1
                package.addfile(member, io.BytesIO(b"x"))
            archive.write_bytes(archive.read_bytes()[:10])
            with self.assertRaisesRegex(ValueError, "invalid runtime package archive"):
                _safe_extract_tar(archive, root / "extract")

    def test_malformed_zip_is_translated_to_package_diagnostic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "malformed.zip"
            archive.write_bytes(b"not a zip archive")
            with self.assertRaisesRegex(ValueError, "invalid artifact archive"):
                _safe_extract_zip(archive, root / "extract")

    def test_corrupt_deflate_zip_is_translated_to_package_diagnostic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "corrupt-deflate.zip"
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as zipped:
                zipped.writestr("codexy-marketplace-plugin.tar.gz", b"x" * 2_048)
            with zipfile.ZipFile(archive) as zipped:
                member = zipped.getinfo("codexy-marketplace-plugin.tar.gz")
                data_offset = member.header_offset + 30 + len(member.filename) + len(member.extra)
            contents = bytearray(archive.read_bytes())
            contents[data_offset] ^= 0xFF
            archive.write_bytes(contents)
            with self.assertRaisesRegex(ValueError, "invalid artifact archive"):
                _safe_extract_zip(archive, root / "extract")

    def test_duplicate_tar_members_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "duplicate.tar.gz"
            with tarfile.open(archive, "w:gz") as package:
                for _ in range(2):
                    member = tarfile.TarInfo("plugins/codexy/plugin.json")
                    member.size = 1
                    package.addfile(member, io.BytesIO(b"x"))
            with self.assertRaisesRegex(ValueError, "duplicate"):
                _safe_extract_tar(archive, root / "extract")

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
            with tarfile.open(archive, "w:gz") as package:
                package.addfile(member, io.BytesIO())

            with self.assertRaisesRegex(ValueError, "link"):
                _safe_extract_tar(archive, root / "extract")


if __name__ == "__main__":
    unittest.main()
