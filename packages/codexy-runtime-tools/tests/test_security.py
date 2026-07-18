import io
import tarfile
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.package import _safe_extract_tar, acquire_package


class ArchiveSecurityTests(unittest.TestCase):
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
