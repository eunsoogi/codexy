"""Indentation-aware, fail-closed Rust workflow job and step contexts."""
from __future__ import annotations

import json
from collections.abc import Callable


def put(mapping: dict[str, object], key: str, value: object) -> None:
    if key in mapping:
        mapping["__invalid__"] = True
    else:
        mapping[key] = value


def job_context(
    lines: list[str],
    mapping: Callable[[str], tuple[str, str] | None],
    scalar: Callable[[str], str],
) -> dict[str, object]:
    lines = normalize_block_runs(lines, mapping)
    job: dict[str, object] = {}
    section = ""
    step: dict[str, object] | None = None
    step_section = ""
    for line in lines:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))
        dash = indent == 6 and line.lstrip().startswith("-")
        entry = mapping(line.strip().removeprefix("-").lstrip())
        if indent == 4 and entry is not None:
            key, value = entry
            section, step, step_section = (key if value == "" else ""), None, ""
            put(job, key, [] if key == "steps" and value == "" else {} if value == "" else scalar(value))
            continue
        if section == "steps" and dash:
            step = {}
            job["steps"].append(step)  # type: ignore[union-attr]
            step_section = ""
        if step is not None and entry is not None and indent in {6, 8, 10}:
            key, value = entry
            if indent == 6 and not dash:
                job["__invalid__"] = True
            elif indent == 8:
                put(step, key, {} if value == "" else scalar(value))
                step_section = key if value == "" else ""
            elif indent == 10 and step_section:
                nested = step.get(step_section)
                if isinstance(nested, dict):
                    put(nested, key, scalar(value))
                else:
                    job["__invalid__"] = True
            elif dash:
                put(step, key, {} if value == "" else scalar(value))
                step_section = key if value == "" else ""
            continue
        if section in {"strategy", "env"} and indent == 6 and entry is not None:
            key, value = entry
            nested = job.get(section)
            if isinstance(nested, dict):
                put(nested, key, {} if value == "" else scalar(value))
            else:
                job["__invalid__"] = True
            if section == "strategy" and key == "matrix" and value == "":
                section = "matrix"
            continue
        if section == "matrix" and indent == 8 and entry is not None:
            matrix = job.get("strategy", {}).get("matrix")  # type: ignore[union-attr]
            if isinstance(matrix, dict):
                put(matrix, entry[0], scalar(entry[1]))
            else:
                job["__invalid__"] = True
            continue
        job["__invalid__"] = True
    return job


def normalize_block_runs(
    lines: list[str],
    mapping: Callable[[str], tuple[str, str] | None],
) -> list[str]:
    normalized: list[str] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        indent = len(line) - len(line.lstrip(" "))
        entry = mapping(line.strip().removeprefix("-").lstrip())
        if indent != 8 or entry is None or entry[0] != "run" or entry[1] not in {"|", "|-", "|+"}:
            normalized.append(line)
            index += 1
            continue
        content: list[str] = []
        index += 1
        while index < len(lines):
            candidate = lines[index]
            candidate_indent = len(candidate) - len(candidate.lstrip(" "))
            if candidate.strip() and candidate_indent <= indent:
                break
            content.append(candidate)
            index += 1
        content_indent = min(
            (len(item) - len(item.lstrip(" ")) for item in content if item.strip()),
            default=indent + 2,
        )
        command = "\n".join(
            item[content_indent:] if item.strip() else "" for item in content
        ).strip()
        normalized.append(f"{' ' * indent}run: {json.dumps(command)}")
    return normalized
