"""Keep the public baseline verifier inside the native Windows CI contract."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


class WindowsMarketplaceBaselineContractTests(unittest.TestCase):
    def test_baseline_verifier_changes_retrigger_the_native_package_job(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        for event in ("pull_request", "push"):
            with self.subTest(event=event):
                block = re.search(
                    rf"(?ms)^  {event}:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
                    workflow,
                )
                self.assertIsNotNone(block, f"{event} trigger is missing")
                assert block is not None
                self.assertIn(
                    '      - ".github/scripts/verify-public-marketplace-baselines.ps1"',
                    block.group("body"),
                )

    def test_candidate_check_preserves_entrypoint_and_avoids_selected_watcher(self):
        repository = Path(__file__).resolve().parents[3]
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("& $env:GETCODEXY_CANDIDATE_RUNTIME --help", workflow)
        self.assertIn("test_core_hook_timing.py", workflow)
        self.assertNotIn("smoke-candidate-mcp-runtime.ps1", workflow)
        self.assertNotIn(
            'Join-Path $env:CODEXY_PUBLIC_BUNDLE_ROOT "plugins/codexy/runtime"',
            workflow,
        )
