"""Archive-root spelling is validated before filesystem extraction."""

import io
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools import package


RUNTIME_NAME = "codexy-mcp-lsp-linux-x86_64.bin"


class RuntimePackageRootSpellingTests(unittest.TestCase):
    def test_noncanonical_roots_are_rejected_before_extraction(self) -> None:
        cases = {
            "case-variant": f"plugins/CODEXY/runtime/{RUNTIME_NAME}",
            "windows-trailing-dot": f"plugins/codexy./runtime/{RUNTIME_NAME}",
            "windows-trailing-space": f"plugins/codexy /runtime/{RUNTIME_NAME}",
            "case-variant-plugins": f"Plugins/codexy/runtime/{RUNTIME_NAME}",
            "windows-separators": rf"plugins\codexy\runtime\{RUNTIME_NAME}",
            "traversal": f"plugins/codexy/../codexy/runtime/{RUNTIME_NAME}",
        }
        for name, member_name in cases.items():
            with self.subTest(case=name), tempfile.TemporaryDirectory() as temporary:
                archive = Path(temporary) / "runtime.tar.gz"
                self.write_archive(archive, {member_name: b"runtime"})
                work = Path(temporary) / "work"
                work.mkdir()
                with (
                    mock.patch.object(package, "_safe_extract_tar") as extract,
                    self.assertRaisesRegex(RuntimeError, "non-canonical|ambiguous"),
                ):
                    package.unpack_runtime(
                        archive=archive,
                        work=work,
                        runtime_name=RUNTIME_NAME,
                    )
                extract.assert_not_called()

    def test_mixed_canonical_roots_are_rejected_before_extraction(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "runtime.tar.gz"
            self.write_archive(
                archive,
                {
                    f"plugins/codexy/runtime/{RUNTIME_NAME}": b"core",
                    f"plugins/codexy-devtools/runtime/{RUNTIME_NAME}": b"devtools",
                },
            )
            work = Path(temporary) / "work"
            work.mkdir()
            with (
                mock.patch.object(package, "_safe_extract_tar") as extract,
                self.assertRaisesRegex(RuntimeError, "mixed plugin roots"),
            ):
                package.unpack_runtime(
                    archive=archive,
                    work=work,
                    runtime_name=RUNTIME_NAME,
                )
            extract.assert_not_called()

    @staticmethod
    def write_archive(path: Path, files: dict[str, bytes]) -> None:
        with tarfile.open(path, "w:gz") as archive:
            for name, contents in files.items():
                member = tarfile.TarInfo(name)
                member.size = len(contents)
                archive.addfile(member, io.BytesIO(contents))


if __name__ == "__main__":
    unittest.main()
