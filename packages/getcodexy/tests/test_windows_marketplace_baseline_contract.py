"""Keep the public baseline verifier inside the native Windows CI contract."""

from __future__ import annotations

import re
import unittest
from pathlib import Path

from scripts import resolve_prior_public_version


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
                self.assertIn(
                    '      - "scripts/resolve_prior_public_version.py"',
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

    def test_unpublished_automatic_target_uses_the_latest_stable_predecessor(self):
        target = resolve_prior_public_version.parse_version("2.0.0")
        self.assertIsNotNone(target)
        assert target is not None

        baseline = resolve_prior_public_version.resolve_package_baseline(
            target, ["v1.12.0", "v1.13.4", "v2.0.0-rc.1", "latest"]
        )

        self.assertEqual(baseline, "1.13.4")

    def test_published_current_target_remains_the_selected_package_baseline(self):
        target = resolve_prior_public_version.parse_version("2.0.0")
        self.assertIsNotNone(target)
        assert target is not None

        baseline = resolve_prior_public_version.resolve_package_baseline(
            target, ["v1.13.4", "v2.0.0"]
        )

        self.assertEqual(baseline, "2.0.0")

    def test_missing_stable_predecessor_fails_closed(self):
        target = resolve_prior_public_version.parse_version("2.0.0")
        self.assertIsNotNone(target)
        assert target is not None

        with self.assertRaisesRegex(
            ValueError, "no published stable release precedes target v2.0.0"
        ):
            resolve_prior_public_version.resolve_package_baseline(
                target, ["v2.0.0-rc.1", "latest"]
            )

    def test_automatic_fallback_is_limited_to_pull_request_and_push_matches(self):
        repository = Path(__file__).resolve().parents[3]
        verifier = (
            repository / "scripts/verify_public_marketplace_bundle.py"
        ).read_text(encoding="utf-8")

        self.assertIn(
            'automatic_event = os.environ.get("GITHUB_EVENT_NAME") in {"pull_request", "push"}',
            verifier,
        )
        self.assertIn(
            "elif automatic_event and (previous_tag is None or previous_tag == current_tag):",
            verifier,
        )
        self.assertIn(
            "elif previous_tag is not None and previous_tag != current_tag:", verifier
        )
        self.assertIn("resolve_automatic_public_baseline", verifier)
        self.assertIn("--prefer-published-target", verifier)

    def test_explicit_override_is_checked_before_automatic_baseline_resolution(self):
        repository = Path(__file__).resolve().parents[3]
        verifier = (
            repository / "scripts/verify_public_marketplace_bundle.py"
        ).read_text(encoding="utf-8")
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        explicit = verifier.index("elif override:")
        automatic = verifier.index(
            "elif automatic_event and (previous_tag is None or previous_tag == current_tag):"
        )

        self.assertLess(explicit, automatic)
        self.assertIn('canonical_version(override, "PRIOR_PUBLIC_VERSION")', verifier)
        self.assertIn('override = os.environ.get("PRIOR_PUBLIC_VERSION", "")', verifier)
        self.assertIn(
            "PRIOR_PUBLIC_VERSION: ${{ inputs.prior_public_version || '' }}", workflow
        )
