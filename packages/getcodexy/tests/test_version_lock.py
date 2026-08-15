from __future__ import annotations

import unittest
from pathlib import Path

from codexy_runtime_tools.version_lock import (
    default_package_version,
    parse_package_version,
)


class VersionLockTests(unittest.TestCase):
    def test_source_checkout_uses_the_canonical_lockfile_resource(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        canonical = repository / "packages/getcodexy/uv.lock"

        self.assertEqual(
            default_package_version(), parse_package_version(canonical.read_text())
        )

    def test_rejects_missing_duplicate_or_malformed_getcodexy_records(self) -> None:
        for lock in (
            '[[package]]\nname = "another"\nversion = "7.8.9"\n',
            "\n".join(
                (
                    "[[package]]",
                    'name = "getcodexy"',
                    'version = "7.8.9"',
                    "[[package]]",
                    'name = "getcodexy"',
                    'version = "7.8.9"',
                )
            ),
            '[[package]]\nname = "getcodexy"\nversion = "7.8"\n',
        ):
            with self.subTest(lock=lock), self.assertRaisesRegex(ValueError, "uv.lock"):
                parse_package_version(lock)


if __name__ == "__main__":
    unittest.main()
