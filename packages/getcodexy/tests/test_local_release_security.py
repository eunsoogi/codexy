"""Archive mode and official-host boundaries for verified local release acquisition."""

import io
import os
from pathlib import Path
import stat
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from codexy_runtime_tools.local_release_bundle import _command
from codexy_runtime_tools.public_marketplace_bundle import safe_extract


class LocalReleaseSecurityTests(unittest.TestCase):
    @unittest.skipIf(os.name == "nt", "POSIX executable and special mode semantics")
    def test_extract_preserves_execute_bits_without_privileged_or_write_modes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive_path = root / "bundle.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                directory = tarfile.TarInfo("runtime")
                directory.type = tarfile.DIRTYPE
                directory.mode = (
                    0o6000  # Missing directory search bits must not strand files.
                )
                archive.addfile(directory)
                for name, mode in (
                    ("executable", 0o6777),
                    ("data", 0o6666),
                    ("owner", 0o4700),
                ):
                    member = tarfile.TarInfo(f"runtime/{name}")
                    member.mode = mode
                    member.size = 4
                    archive.addfile(member, io.BytesIO(b"test"))
            destination = root / "extracted"
            safe_extract(archive_path, destination)
            self.assertEqual(
                stat.S_IMODE((destination / "runtime").stat().st_mode), 0o700
            )
            for name, expected in (
                ("executable", 0o711),
                ("data", 0o600),
                ("owner", 0o700),
            ):
                path = destination / "runtime" / name
                self.assertEqual(path.read_bytes(), b"test")
                self.assertEqual(stat.S_IMODE(path.stat().st_mode), expected)

    def test_release_commands_pin_public_github_host_without_mutating_environment(self):
        with (
            patch.dict(os.environ, {"GH_HOST": "enterprise.example.invalid"}),
            patch(
                "codexy_runtime_tools.local_release_bundle.subprocess.run",
                return_value=subprocess.CompletedProcess([], 0, "{}", ""),
            ) as run,
        ):
            self.assertEqual(_command(["gh", "api", "repos/eunsoogi/codexy"]), {})
            self.assertEqual(run.call_args.kwargs["env"]["GH_HOST"], "github.com")
            self.assertEqual(os.environ["GH_HOST"], "enterprise.example.invalid")
