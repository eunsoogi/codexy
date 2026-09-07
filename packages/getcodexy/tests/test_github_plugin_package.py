from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[3]
CORE = ROOT / "plugins" / "codexy"
GITHUB = ROOT / "plugins" / "codexy-github"


class GithubPluginPackageTests(unittest.TestCase):
    def test_core_only_has_no_generic_github_workflow(self) -> None:
        self.assertFalse((CORE / "skills" / "git-workflow").exists())
        self.assertFalse((CORE / "agents" / "codexy-weaver.toml").exists())
        self.assertFalse((CORE / "hooks" / "codexy-issue-title-check.sh").exists())
        self.assertTrue((CORE / "hooks/codexy_policy/envelope.py").is_file())

    def test_github_plugin_declares_the_public_core_dependency(self) -> None:
        manifest = json.loads(
            (GITHUB / ".codex-plugin" / "plugin.json").read_text(encoding="utf-8")
        )
        self.assertEqual(manifest["name"], "codexy-github")
        self.assertNotIn("dependencies", manifest)
        self.assertEqual(
            json.loads(
                (
                    ROOT
                    / "packages"
                    / "getcodexy"
                    / "contracts"
                    / "component-installation-contract.json"
                ).read_text(encoding="utf-8")
            )["dependencies"]["github"],
            ["core"],
        )

    def test_workflow_guidance_respects_user_and_repository_choice(self) -> None:
        skill = (GITHUB / "skills/git-workflow/SKILL.md").read_text(encoding="utf-8")
        issue = (GITHUB / "skills/git-workflow/references/issue-intake.md").read_text(
            encoding="utf-8"
        )
        pull_request = (
            GITHUB / "skills/git-workflow/references/pr-review-and-handoff.md"
        ).read_text(encoding="utf-8")
        agent = (GITHUB / "skills/git-workflow/agents/openai.yaml").read_text(
            encoding="utf-8"
        )

        self.assertIn("Ordinary authorized GitHub metadata", skill)
        self.assertIn("retains the existing issue-title", " ".join(skill.split()))
        self.assertIn("retains this existing issue-title check", issue)
        self.assertIn("retains its existing PR-title", pull_request)
        self.assertIn("squash-subject check", pull_request)
        self.assertIn(
            "no heading or footer is required by this plugin",
            " ".join(issue.split()),
        )
        self.assertIn("ordinary authorized GitHub metadata", pull_request)
        self.assertIn("MUST NOT be blocked, rewritten", pull_request)
        self.assertIn("only when the operation is issue-sized", agent)

    def test_copied_package_preserves_generic_hook_and_specialist_artifacts(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            installed = Path(directory) / "codexy-github"
            shutil.copytree(GITHUB, installed)
            hooks = installed / "hooks"
            self._run(
                hooks / "codexy-issue-title-check.sh",
                "--issue-title",
                "Extract GitHub workflow",
            )
            self._run(
                hooks / "codexy-pr-title-check.sh",
                "--pr-title",
                "refactor(github): extract workflow",
            )
            state = installed / "review-state.json"
            state.write_text(
                json.dumps(
                    {
                        "number": 553,
                        "state": "OPEN",
                        "repository": "owner/repo",
                        "labels": [{"name": "type/refactor"}],
                        "repositoryLabels": [{"name": "type/refactor"}],
                    }
                ),
                encoding="utf-8",
            )
            self._run(hooks / "codexy-pr-label-check.sh", "--pr-state-file", str(state))
            self._run(
                hooks / "codexy-merge-message-check.sh",
                "--expected-issue",
                "553",
                "--expected-pr",
                "554",
                "--merge-message",
                "refactor(github): extract workflow (#554)\n\nFixes #553\n",
            )
            home = installed / "codex-home"
            sentinel = home / "agents/codexy/codexy-sentinel.toml"
            sentinel.parent.mkdir(parents=True)
            sentinel.write_text(
                '# CODEXY MANAGED AGENT\nname = "codexy-sentinel"\n',
                encoding="utf-8",
            )
            bridge = (
                installed
                / "skills/git-workflow/scripts/bootstrap_codexy_github_agent.py"
            )
            self._run(bridge, "--codex-home", str(home))
            self._run(bridge, "--codex-home", str(home), "--diagnose")
            projected = home / "agents/codexy-github/codexy-weaver.toml"
            self.assertIn(
                'name = "codexy-weaver"', projected.read_text(encoding="utf-8")
            )

    @staticmethod
    def _run(path: Path, *args: str) -> None:
        result = subprocess.run(
            [str(path), *args], check=False, capture_output=True, text=True
        )
        if result.returncode:
            raise AssertionError(f"{path.name} failed:\n{result.stdout}{result.stderr}")


if __name__ == "__main__":
    unittest.main()
