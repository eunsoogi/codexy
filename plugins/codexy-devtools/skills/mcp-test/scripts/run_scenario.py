#!/usr/bin/env python3
"""Run or compare declarative MCP scenarios from an installed Devtools plugin."""

from __future__ import annotations

import argparse
import copy
import json
import sys
from collections.abc import Mapping
from enum import Enum
from pathlib import Path
from typing import Any


_PLUGIN_ROOT = Path(__file__).resolve().parents[3]
_SCENARIO_ROOT = _PLUGIN_ROOT / "scripts"
if not _SCENARIO_ROOT.is_dir():
    raise SystemExit("mcp-test is missing the bundled scenario producers")
sys.path.insert(0, str(_SCENARIO_ROOT))

import scenario_core as _scenario_core  # noqa: E402
import scenario_flow as _scenario_flow  # noqa: E402
import scenario_compare as _scenario_compare  # noqa: E402
from scenario_compare import ScenarioExecutor, compare_scenario  # noqa: E402
from scenario_core import ExpectedResult, ResultKind, SingleCall  # noqa: E402
from scenario_flow import DataReference, Scenario, ScenarioStep, run_scenario  # noqa: E402


_TYPE_SPECS = {
    "string": str,
    "integer": int,
    "number": (int, float),
    "boolean": bool,
    "object": dict,
    "array": list,
}


def _mapping(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be an object")
    return value


def _load_spec(path: Path) -> dict[str, Any]:
    try:
        data = _mapping(json.loads(path.read_text(encoding="utf-8")), "scenario")
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read scenario: {error}") from error
    if not isinstance(data.get("id"), str) or not data["id"]:
        raise ValueError("scenario id must be a non-empty string")
    steps = data.get("steps")
    if not isinstance(steps, list) or not steps:
        raise ValueError("scenario steps must be a non-empty array")
    target_names: set[str] | None = None
    for raw_step in steps:
        step = _mapping(raw_step, "step")
        targets = _mapping(step.get("targets"), "step targets")
        current_targets = set(targets)
        if not current_targets or any(not item for item in current_targets):
            raise ValueError("step targets must have non-empty names")
        if target_names is None:
            target_names = current_targets
        elif current_targets != target_names:
            raise ValueError("every step must declare the same target names")
    data["_target_names"] = tuple(sorted(target_names or ()))
    return data


def _expected(value: Any) -> ExpectedResult:
    raw = _mapping(value or {}, "expected")
    return ExpectedResult(
        ResultKind(raw.get("kind", ResultKind.SUCCESS.value)), raw.get("fields", {})
    )


def _reference(value: Any) -> DataReference:
    raw = _mapping(value, "reference")
    expected_type = raw.get("type")
    if expected_type is not None:
        if expected_type not in _TYPE_SPECS:
            raise ValueError("reference type is unsupported")
        expected_type = _TYPE_SPECS[expected_type]
    return DataReference(raw.get("step", ""), raw.get("path", ""), expected_type)


def _scenario(spec: dict[str, Any], target: str) -> Scenario:
    if target not in spec["_target_names"]:
        raise ValueError(f"unknown target: {target}")
    steps = []
    for raw_step in spec["steps"]:
        step = _mapping(raw_step, "step")
        config = _mapping(step["targets"][target], f"target {target}")
        call = SingleCall(
            argv=tuple(config["argv"]),
            cwd=Path(config["cwd"]),
            environment=config.get("environment", {}),
            tool=step["tool"],
            allowed_tools=frozenset(step["allowed_tools"]),
            arguments=step.get("arguments", {}),
            stored_fields=step["stored_fields"],
            expected=_expected(step.get("expected")),
            protocol_version=step.get(
                "protocol_version", _scenario_core.DEFAULT_PROTOCOL_VERSION
            ),
            transport=step.get("transport", _scenario_core.DEFAULT_TRANSPORT),
            timeout_seconds=step.get("timeout_seconds", 5.0),
            output_limit_bytes=step.get("output_limit_bytes", 64 * 1024),
        )
        references = {
            path: _reference(value)
            for path, value in step.get("references", {}).items()
        }
        steps.append(ScenarioStep(step["name"], call, references))
    return Scenario(tuple(steps), spec.get("deadline_seconds", 30.0))


def _normalizers(spec: dict[str, Any]):
    values = spec.get("normalizers", [])
    if not isinstance(values, list):
        raise ValueError("normalizers must be an array")
    declared = {
        (step["name"], field)
        for step in spec["steps"]
        for field in step["stored_fields"]
    }
    result = {}
    for raw in values:
        item = _mapping(raw, "normalizer")
        key = (item.get("step"), item.get("field"))
        if key not in declared:
            raise ValueError("normalizer must select a declared stored field")
        if item.get("normalizer") != "constant" or "value" not in item:
            raise ValueError("the only supported normalizer is constant with value")
        constant = copy.deepcopy(item["value"])
        result[key] = lambda _value, constant=constant: copy.deepcopy(constant)
    return result


def _json_safe(value: Any) -> Any:
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, Mapping):
        return {key: _json_safe(item) for key, item in value.items()}
    if isinstance(value, (tuple, list)):
        return [_json_safe(item) for item in value]
    fields = getattr(value, "__dataclass_fields__", None)
    if fields is not None:
        return {name: _json_safe(getattr(value, name)) for name in fields}
    return value


def _run_dict(target: str, result) -> dict[str, Any]:
    return {
        "schema": "codexy.mcp-test.run.v1",
        "target": target,
        "status": result.status,
        "ok": result.ok,
        "failed_step": result.failed_step,
        "elapsed_seconds": result.elapsed_seconds,
        "steps": [{**_json_safe(step), "status": step.status} for step in result.steps],
    }


def _provenance() -> dict[str, Any]:
    return {
        "cli": str(Path(__file__).resolve()),
        "producer_root": str(_SCENARIO_ROOT.resolve()),
        "producer_modules": [
            str(Path(module.__file__).resolve())
            for module in (_scenario_core, _scenario_flow, _scenario_compare)
        ],
    }


def _emit(value: dict[str, Any], code: int) -> int:
    value["implementation"] = _provenance()
    print(json.dumps(value, sort_keys=True))
    return code


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("support", help="print the support contract")
    run = commands.add_parser("run", help="run one selected target")
    run.add_argument("--scenario", required=True, type=Path)
    run.add_argument("--target", required=True)
    compare = commands.add_parser("compare", help="compare two selected targets")
    compare.add_argument("--scenario", required=True, type=Path)
    compare.add_argument("--baseline-target", required=True)
    compare.add_argument("--candidate-target", required=True)
    return parser


def main(arguments: list[str] | None = None) -> int:
    args = _parser().parse_args(arguments)
    if args.command == "support":
        return _emit(
            {
                "schema": "codexy.mcp-test.support.v1",
                "support": _scenario_core.supported_versions(),
            },
            0,
        )
    try:
        _scenario_core.validate_platform()
        spec = _load_spec(args.scenario)
        if args.command == "run":
            result = run_scenario(_scenario(spec, args.target))
            return _emit(_run_dict(args.target, result), 0 if result.ok else 2)
        baseline = args.baseline_target
        candidate = args.candidate_target
        if baseline == candidate:
            raise ValueError("baseline and candidate targets must differ")
        comparison = compare_scenario(
            _scenario(spec, baseline),
            ScenarioExecutor(
                baseline, lambda _input: run_scenario(_scenario(spec, baseline))
            ),
            ScenarioExecutor(
                candidate, lambda _input: run_scenario(_scenario(spec, candidate))
            ),
            scenario_id=spec["id"],
            normalizers=_normalizers(spec),
        )
        report = comparison.to_dict()
        report.update(
            {
                "schema": "codexy.mcp-test.compare.v1",
                "baseline_target": baseline,
                "candidate_target": candidate,
            }
        )
        return _emit(report, comparison.exit_code)
    except (AttributeError, KeyError, OSError, TypeError, ValueError) as error:
        return _emit(
            {
                "schema": "codexy.mcp-test.error.v1",
                "status": "error",
                "error": str(error),
            },
            2,
        )


if __name__ == "__main__":
    raise SystemExit(main())
