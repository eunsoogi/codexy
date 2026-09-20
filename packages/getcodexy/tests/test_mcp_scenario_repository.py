"""Repository-tool provenance and fresh Devtools package boundary proof."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPOSITORY = Path(__file__).resolve().parents[3]
REPOSITORY_TOOLS = REPOSITORY / ".agents/skills/mcp-test"
CLI = Path("scripts/run_scenario.py")
NORMALIZER = dict(
    step="search", field="timestamp", normalizer="constant", value="<normalized>"
)
FIXTURE = r"""
import json, sys
options = dict(zip(sys.argv[1::2], sys.argv[2::2]))
step, variant, record = (options[key] for key in ("--step", "--variant", "--record"))
def send(identifier, result): print(json.dumps({"jsonrpc": "2.0", "id": identifier, "result": result}), flush=True)
for line in sys.stdin:
    request = json.loads(line); identifier = request.get("id"); method = request.get("method")
    if method == "initialize": send(identifier, {"protocolVersion": "2024-11-05", "serverInfo": {"name": "installed-fixture"}}); continue
    if method == "notifications/initialized": continue
    if method == "tools/list": send(identifier, {"tools": [{"name": "search"}, {"name": "detail"}]}); continue
    if method == "tools/call":
        arguments = request.get("params", {}).get("arguments", {})
        with open(record, "w", encoding="utf-8") as output: json.dump(arguments, output)
        if step == "search":
            data = {"id": "item-42", "timestamp": "candidate" if variant == "candidate-different" else "same"}
        else:
            data = {"matched_id": "wrong-id" if variant == "candidate-bad-id" else arguments.get("id")}
        send(identifier, {"content": [{"type": "text", "text": step}], "data": data}); break
"""


class RepositoryScenarioTests(unittest.TestCase):
    def _copy_repository_tools(self, root: Path) -> Path:
        tools = root / "repository-tools" / "mcp-test"
        shutil.copytree(REPOSITORY_TOOLS, tools)
        return tools

    def _scenario(self, work: Path, variants: dict[str, str]) -> dict:
        def target(step, name, variant):
            argv = [
                sys.executable,
                "-c",
                FIXTURE,
                "--step",
                step,
                "--variant",
                variant,
                "--record",
                str(work / f"{name}-{step}.json"),
            ]
            return {
                "argv": argv,
                "cwd": str(work),
                "environment": {"SCENARIO_MARKER": "installed"},
            }

        def step(name, tool, stored, expected, arguments, references=None):
            result = {
                "name": name,
                "tool": tool,
                "allowed_tools": ["search", "detail"],
                "arguments": arguments,
                "stored_fields": stored,
                "expected": expected,
                "targets": {
                    key: target(name, key, value) for key, value in variants.items()
                },
            }
            if references:
                result["references"] = references
            return result

        return {
            "id": "installed-search-detail",
            "deadline_seconds": 10,
            "steps": [
                step(
                    "search",
                    "search",
                    {"id": "/result/data/id", "timestamp": "/result/data/timestamp"},
                    {"kind": "success", "fields": {"/result/data/id": "item-42"}},
                    {"query": "codex"},
                ),
                step(
                    "detail",
                    "detail",
                    {"matched_id": "/result/data/matched_id"},
                    {
                        "kind": "success",
                        "fields": {"/result/data/matched_id": "item-42"},
                    },
                    {"id": None},
                    {"/id": {"step": "search", "path": "/id", "type": "string"}},
                ),
            ],
        }

    def _run(self, root, tools, arguments):
        environment = os.environ.copy()
        for name in ("PYTHONPATH", "PYTHONHOME"):
            environment.pop(name, None)
        self.assertFalse({"PYTHONPATH", "PYTHONHOME"} & environment.keys())
        return subprocess.run(
            [sys.executable, str(tools / CLI), *arguments],
            cwd=root / "outside",
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )

    def _fixture(self, root, variants):
        work = root / "work"
        (root / "outside").mkdir()
        work.mkdir()
        scenario = work / "scenario.json"
        scenario.write_text(
            json.dumps(self._scenario(work, variants)), encoding="utf-8"
        )
        return work, scenario

    def test_repository_tool_runs_chain_and_reports_provenance(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = self._copy_repository_tools(root)
            work, scenario = self._fixture(root, {"baseline": "baseline"})
            result = self._run(
                root,
                tools,
                ["run", "--scenario", str(scenario), "--target", "baseline"],
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertEqual(report["status"], "success")
            self.assertEqual(
                report["steps"][1]["execution"]["stored"]["matched_id"], "item-42"
            )
            observed = json.loads(
                (work / "baseline-detail.json").read_text(encoding="utf-8")
            )
            self.assertEqual(observed["id"], "item-42")
            implementation = report["implementation"]
            tools_root = tools.resolve()
            self.assertEqual(implementation["surface"], "repository-only")
            for path in [
                implementation["cli"],
                implementation["producer_root"],
                *implementation["producer_modules"],
            ]:
                self.assertTrue(Path(path).is_relative_to(tools_root))
                self.assertFalse(Path(path).is_relative_to(REPOSITORY.resolve()))
            self.assertTrue((tools / "SKILL.md").is_file())

    def test_compare_codes_and_explicit_normalizer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = self._copy_repository_tools(root)
            _work, scenario = self._fixture(
                root, {"baseline": "baseline", "candidate": "candidate-different"}
            )
            arguments = ["compare", "--scenario", str(scenario)]
            arguments += [
                "--baseline-target",
                "baseline",
                "--candidate-target",
                "candidate",
            ]
            different = self._run(root, tools, arguments)
            self.assertEqual(different.returncode, 1)
            self.assertEqual(json.loads(different.stdout)["status"], "different")

            data = json.loads(scenario.read_text(encoding="utf-8"))
            data["normalizers"] = [NORMALIZER]
            scenario.write_text(json.dumps(data), encoding="utf-8")
            matching = self._run(root, tools, arguments)
            self.assertEqual(matching.returncode, 0)
            self.assertEqual(json.loads(matching.stdout)["status"], "match")

            data["normalizers"] = []
            for step in data["steps"]:
                step["targets"]["candidate"]["argv"][6] = "candidate-bad-id"
            scenario.write_text(json.dumps(data), encoding="utf-8")
            regression = self._run(root, tools, arguments)
            report = json.loads(regression.stdout)
            self.assertEqual(regression.returncode, 2)
            self.assertEqual(report["status"], "incomparable")
            self.assertEqual(report["candidate"]["failed_step"], "detail")

    def test_support_and_simulated_windows_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = self._copy_repository_tools(root)
            work, scenario = self._fixture(root, {"baseline": "baseline"})
            support = self._run(root, tools, ["support"])
            contract = json.loads(support.stdout)["support"]
            self.assertEqual(contract["protocol_versions"], ["2024-11-05"])
            self.assertEqual(contract["platforms"], ["posix"])

            wrapper = root / "simulate_windows.py"
            wrapper.write_text(
                "import runpy, sys\n"
                "from pathlib import Path\n"
                "cli = Path(sys.argv[1])\n"
                "sys.path.insert(0, str(cli.parent))\n"
                "import scenario_core.support\n"
                "sys.argv = [str(cli), *sys.argv[2:]]\n"
                "from unittest.mock import patch\n"
                "with patch('scenario_core.validate_platform', side_effect=ValueError('unsupported execution platform: nt')):\n"
                "    runpy.run_path(str(cli), run_name='__main__')\n",
                encoding="utf-8",
            )
            environment = {"PATH": os.environ.get("PATH", "")}
            blocked = subprocess.run(
                [
                    sys.executable,
                    str(wrapper),
                    str(tools / CLI),
                    "run",
                    "--scenario",
                    str(scenario),
                    "--target",
                    "baseline",
                ],
                cwd=root / "outside",
                env=environment,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(blocked.returncode, 2)
            self.assertIn(
                "unsupported execution platform", json.loads(blocked.stdout)["error"]
            )
            self.assertFalse((work / "baseline-search.json").exists())


if __name__ == "__main__":
    unittest.main()
