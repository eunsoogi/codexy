from __future__ import annotations

import re
import tomllib
import unittest
from pathlib import Path
from subprocess import run
from tempfile import TemporaryDirectory

from codexy_runtime_tools.component_integrity import COMPONENT_FILES, verify_component
from codexy_runtime_tools import updater
from codexy_runtime_tools.github_pre_session import run_github_pre_session
from codexy_runtime_tools.pre_session import run_pre_session


class PublicActivationContractTests(unittest.TestCase):
    def test_github_component_has_a_public_dependency_aware_activation_command(
        self,
    ) -> None:
        repository = Path(__file__).resolve().parents[3]
        metadata = (repository / "packages/getcodexy/pyproject.toml").read_text(
            encoding="utf-8"
        )
        scripts = tomllib.loads(metadata)["project"]["scripts"]

        self.assertEqual(
            scripts["codexy-github-install"],
            "codexy_runtime_tools.github_pre_session:main",
        )
        self.assertEqual(
            scripts["codexy-github-check"],
            "codexy_runtime_tools.github_checks:main",
        )
        self.assertTrue(callable(run_github_pre_session))
        self.assertFalse((repository / "install").exists())
        workflow = (repository / ".github/workflows/python-package.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("github-activation-windows", workflow)
        self.assertIn("Run native component lifecycle tests", workflow)
        for test in (
            "test_component_cli.py",
            "test_component_lifecycle.py",
            "test_component_lifecycle_version_admission.py",
            "test_component_lifecycle_interrupt.py",
            "test_component_lifecycle_journal.py",
            "test_component_transition_model.py",
            "test_component_lifecycle_finalization.py",
            "test_component_lifecycle_preflight.py",
            "test_component_lifecycle_update_recovery.py",
            "test_component_lifecycle_admission.py",
            "test_component_transaction_durability.py",
        ):
            self.assertIn(test, workflow)
        self.assertNotIn("-p 'test_component*.py'", workflow)
        self.assertNotIn("test_component_integrity_windows.py", workflow)
        self.assertNotIn("test_component_manifest_resolver.py", workflow)
        self.assertIn("getcodexy.exe --help", workflow)
        self.assertIn("codexy-github-install.exe --help", workflow)
        self.assertIn("test_version_lock.py", workflow)
        self.assertIn("default_package_version", workflow)
        self.assertIn(
            '$env:PYTHONPATH = "packages/getcodexy/tests"\n'
            "          .package-venv\\Scripts\\python -m unittest "
            "packages/getcodexy/tests/test_component_distribution.py",
            workflow,
        )
        self.assertIn("codexy-github-check.exe --check-pr-labels", workflow)
        self.assertIn("& (Join-Path $hookRoot", workflow)
        self.assertNotIn("cmd /d /s /c", workflow)
        self.assertIn('"plugins/codexy-github/**"', workflow)

    @staticmethod
    def _windows_activation_pwsh_runs(workflow: str) -> list[str]:
        job = re.search(
            r"(?ms)^  github-activation-windows:\n(?P<body>.*?)(?=^  \S.*:\n|\Z)",
            workflow,
        )
        if job is None:
            raise AssertionError("github-activation-windows job is missing")
        runs = []
        for step in re.finditer(
            r"(?ms)^      - name: [^\n]+\n(?P<body>.*?)(?=^      - name:|\Z)",
            job.group("body"),
        ):
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

    def test_source_only_updater_remains_unpublished(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        metadata = (repository / "packages/getcodexy/pyproject.toml").read_text(
            encoding="utf-8"
        )
        scripts = tomllib.loads(metadata)["project"]["scripts"]
        activation_pattern = re.compile(
            r"\buvx\s+--from\s+getcodexy(?:==[^\s]+)?\s+codexy-update\s+--pre-session\b"
        )

        self.assertFalse((repository / "install").exists())
        self.assertNotIn("codexy-update", scripts)
        for path in (
            repository / "README.md",
            repository / "README.ko.md",
            repository / ".github/workflows/python-package.yml",
            repository / "packages/getcodexy/src/codexy_runtime_tools/runtime.py",
            repository / "plugins/codexy/check-codexy-agents",
            repository
            / "plugins/codexy/skills/orchestration/references/agent-registration.md",
        ):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("chmod +x install && ./install", text)
            self.assertNotIn("run the root installer", text)
            self.assertNotIn("codexy-update", text)
            self.assertIsNone(activation_pattern.search(text))

        self.assertTrue(callable(updater.main))
        self.assertTrue(callable(run_pre_session))

    # fmt: off
    def test_manifest_hashed_integrity_inputs_require_lf(self) -> None:
        repo, paths = Path(__file__).resolve().parents[3], tuple(Path("plugins") / component / relative for component, files in COMPONENT_FILES.items() for relative in (*files, ".codex-plugin/plugin.json"))
        self.assertTrue({path.suffix for path in paths} >= {".py", ".json", ".cmd"})
        with TemporaryDirectory() as temporary:
            run(["git", "clone", "--no-local", "--config", "core.autocrlf=true", repo, (checkout := Path(temporary) / "checkout")], check=True, capture_output=True, text=True)
            self.assertTrue((fields := run(["git", "-C", checkout, "check-attr", "-z", "eol", "--", *paths], check=True, capture_output=True).stdout.split(b"\0"))[-1] == b"" and len(fields) == len(paths) * 3 + 1 and all(fields[:-1:3]) and fields[1:-1:3] == [b"eol"] * len(paths) and fields[2:-1:3] == [b"lf"] * len(paths))
            self.assertTrue(all(b"\r\n" not in (checkout / path).read_bytes() for path in paths) and all(verify_component(checkout / "plugins" / component, component) for component in COMPONENT_FILES))
            (mutated := checkout / (py_path := next(path for path in paths if path.suffix == ".py"))).write_bytes(mutated.read_bytes().replace(b"\n", b"\r\n"))
            self.assertRaisesRegex(ValueError, "component integrity mismatch", verify_component, checkout / "plugins" / py_path.parts[1], py_path.parts[1])


# fmt: on


if __name__ == "__main__":
    unittest.main()
