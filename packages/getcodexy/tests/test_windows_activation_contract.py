import re
import unittest
from pathlib import Path


class WindowsActivationContractTests(unittest.TestCase):
    @staticmethod
    def _windows_activation_pwsh_runs(
        workflow: str,
        names: tuple[str, ...] = (
            "Run native component lifecycle tests",
            "Test dependency-aware GitHub activation",
        ),
    ) -> list[str]:
        job = re.search(
            r"(?ms)^  github-activation-windows:\n(?P<body>.*?)(?=^  \S.*:\n|\Z)",
            workflow,
        )
        if job is None:
            raise AssertionError("github-activation-windows job is missing")
        runs = []
        for step in re.finditer(
            r"(?ms)^      - name: (?P<name>[^\n]+)\n(?P<body>.*?)(?=^      - name:|\Z)",
            job.group("body"),
        ):
            if step.group("name") not in names:
                continue
            body = step.group("body")
            if not re.search(r"^        shell: pwsh$", body, re.MULTILINE):
                continue
            run = re.search(
                r"(?ms)^        run: \|\n(?P<run>(?:^          .*(?:\n|\Z))*)",
                body,
            )
            if run is None:
                raise AssertionError("PowerShell step has no run block")
            runs.append("\n".join(line[10:] for line in run.group("run").splitlines()))
        return runs

    def _assert_windows_native_commands_fail_fast(self, runs: list[str]) -> None:
        self.assertEqual(len(runs), 2)
        source_import = (
            '$env:PYTHONPATH = "packages/getcodexy/src;packages/getcodexy/tests"'
        )
        for run in runs:
            self.assertIn('$ErrorActionPreference = "Stop"', run)
            self.assertIn(source_import, run)
        self.assertIn(
            '$env:PYTHONPATH = "packages/getcodexy/tests"',
            next(run for run in runs if "test_component_distribution.py" in run),
        )
        native = re.compile(
            r"(?:^(?:python(?:\.exe)?(?=\s|$)|\.package-venv\\Scripts\\"
            r"(?:python(?:\.exe)?(?=\s|$)|getcodexy\.exe\b|"
            r"codexy-github-install\.exe\b|codexy-github-check\.exe\b))"
            r"|=\s*python(?:\.exe)?(?=\s|$)|& \(Join-Path \$hookRoot)"
        )
        for run in runs:
            lines = run.splitlines()
            for index, line in enumerate(lines):
                if not native.search(line.strip()):
                    continue
                next_line = next(
                    (
                        candidate.strip()
                        for candidate in lines[index + 1 :]
                        if candidate.strip()
                    ),
                    "",
                )
                self.assertEqual(
                    next_line,
                    "if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
                    f"native command is not fail-fast: {line.strip()}",
                )

    def test_windows_activation_propagates_native_failures(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        self._assert_windows_native_commands_fail_fast(
            self._windows_activation_pwsh_runs(workflow)
        )

    def test_windows_activation_contract_rejects_failure_and_scope_mutations(
        self,
    ) -> None:
        repository = Path(__file__).resolve().parents[3]
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        mutations = (
            (
                "source import path",
                '$env:PYTHONPATH = "packages/getcodexy/src;packages/getcodexy/tests"',
                '$env:PYTHONPATH = "packages/getcodexy/src"',
            ),
            (
                "first PowerShell error preference",
                '$ErrorActionPreference = "Stop"',
                '$ErrorActionPreference = "Continue"',
            ),
            (
                "Python check",
                "python -m unittest @componentTests\n"
                "          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
                "python -m unittest @componentTests",
            ),
            (
                "entrypoint check",
                ".package-venv\\Scripts\\getcodexy.exe --help\n"
                "          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
                ".package-venv\\Scripts\\getcodexy.exe --help",
            ),
            (
                "CMD check",
                '$context = \'{"prompt":"Open a GitHub issue"}\' | & '
                '(Join-Path $hookRoot "codexy-github-workflow-context.cmd")\n'
                "          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
                '$context = \'{"prompt":"Open a GitHub issue"}\' | & '
                '(Join-Path $hookRoot "codexy-github-workflow-context.cmd")',
            ),
        )
        for label, old, new in mutations:
            with self.subTest(label=label):
                with self.assertRaises(AssertionError):
                    self._assert_windows_native_commands_fail_fast(
                        self._windows_activation_pwsh_runs(
                            workflow.replace(old, new, 1)
                        )
                    )

        unrelated_job = workflow + (
            "\n  unrelated-pwsh-job:\n"
            "    steps:\n"
            "      - name: Unrelated command\n"
            "        shell: pwsh\n"
            "        run: |\n"
            "          python -m unittest missing_test.py\n"
        )
        self._assert_windows_native_commands_fail_fast(
            self._windows_activation_pwsh_runs(unrelated_job)
        )

    def test_windows_checkout_verification_is_separate_from_lifecycle(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        runs = self._windows_activation_pwsh_runs(
            workflow, ("Verify exact checked-out head",)
        )
        self.assertEqual(len(runs), 1)
        self.assertIn("$env:EXPECTED_HEAD_SHA -notmatch '^[0-9a-f]{40}$'", runs[0])
        self.assertIn("(git rev-parse HEAD).Trim() -ne $env:EXPECTED_HEAD_SHA", runs[0])
        self.assertIn(
            'throw "checked out source commit does not match requested head"', runs[0]
        )

    def test_windows_lifecycle_uses_a_verified_selected_public_release(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        run = self._windows_activation_pwsh_runs(workflow)[0]
        self.assertIn(
            "$verified = python scripts/verify_public_marketplace_bundle.py --output-dir $bundleRoot",
            run,
        )
        self.assertIn("$release = $verified | ConvertFrom-Json", run)
        self.assertIn("$binary = $release.watcher_binary", run)
        self.assertIn("$env:CODEXY_TEST_WATCHER_BINARY = $binary", run)
        self.assertIn(
            '"CODEXY_TEST_WATCHER_BINARY=$binary" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append',
            run,
        )
        self.assertEqual(
            run.splitlines().count("$env:CODEXY_TEST_WATCHER_BINARY = $binary"),
            1,
        )
        self.assertEqual(
            run.splitlines().count(
                '"CODEXY_TEST_WATCHER_BINARY=$binary" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append'
            ),
            1,
        )
        for forbidden in (
            "gh release",
            "Invoke-WebRequest",
            "Invoke-RestMethod",
            "Start-BitsTransfer",
            "releases/download/",
        ):
            self.assertNotIn(forbidden, run)
        self.assertIn("BASE_SHA:", workflow)
        self.assertIn("PRIOR_PUBLIC_VERSION:", workflow)
        self.assertNotIn('$version = "1.7.0"', run)
        helper = (repository / "scripts/verify_public_marketplace_bundle.py").read_text(
            encoding="utf-8"
        )
        for required in (
            "from public_marketplace_bundle_support import",
            "CONTRACT,",
            "release_tag = current_tag",
            '"gh",',
            '"release",',
            '"download",',
            '"runtime-release-receipt.json"',
        ):
            self.assertIn(required, helper)
        support = (
            repository / "scripts/public_marketplace_bundle_support.py"
        ).read_text(encoding="utf-8")
        for required in (
            'CONTRACT = Path(".agents/plugins/release-publish-contract.json")',
            "runtime-release-receipt/v2",
            "manifestSha256",
            "tarfile.open",
            "archive.extract",
        ):
            self.assertIn(required, support)
