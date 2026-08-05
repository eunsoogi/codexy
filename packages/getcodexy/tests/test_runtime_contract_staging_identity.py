"""Candidate staging identity values must be positive JSON integers."""

import hashlib
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

try:
    from .test_runtime_contract import RuntimeContractTests, candidate, encoded, release
except ImportError:
    from test_runtime_contract import RuntimeContractTests, candidate, encoded, release


class StagingIdentityTests(unittest.TestCase):
    def test_rejects_non_positive_or_non_integer_staging_identity_values(self) -> None:
        self.assertTrue(self.verify("stagingRunId", 1))
        self.assertTrue(self.verify("stagingRunAttempt", 1))
        for field in ("stagingRunId", "stagingRunAttempt"):
            for value in (True, False, 0, 1.5, "1", None):
                with self.subTest(field=field, value=value):
                    with self.assertRaises(ValueError):
                        self.verify(field, value)

    @staticmethod
    def verify(field: str, value: object) -> bool:
        harness = RuntimeContractTests("runTest")
        embedded = candidate()
        embedded["artifact"][field] = value  # type: ignore[index]
        document = release()
        document["artifact"]["payloadManifestSha256"] = hashlib.sha256(encoded(embedded)).hexdigest()  # type: ignore[index]
        root, parsed = harness.load(document)
        try:
            with tempfile.TemporaryDirectory() as directory:
                archive = Path(directory) / "runtime.tar.gz"
                harness.archive(archive, embedded)
                return parsed.verify_archive(archive, platform="linux-x86_64")
        finally:
            harness.doCleanups()
