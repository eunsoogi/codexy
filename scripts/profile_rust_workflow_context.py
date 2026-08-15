"""Indentation-aware, fail-closed Rust workflow job and step contexts."""

from __future__ import annotations

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
    job: dict[str, object] = {}
    section = ""
    step: dict[str, object] | None = None
    step_section = ""
    matrix_sequence: list[str] | None = None
    for line in lines:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))
        dash = indent == 6 and line.lstrip().startswith("-")
        entry = mapping(line.strip().removeprefix("-").lstrip())
        if indent == 4 and entry is not None:
            key, value = entry
            section, step, step_section = (key if value == "" else ""), None, ""
            put(
                job,
                key,
                []
                if key == "steps" and value == ""
                else {}
                if value == ""
                else scalar(value),
            )
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
                if entry[1] == "[":
                    matrix_sequence = []
                    put(matrix, entry[0], matrix_sequence)
                else:
                    put(matrix, entry[0], scalar(entry[1]))
            else:
                job["__invalid__"] = True
            continue
        if section == "matrix" and matrix_sequence is not None:
            value = line.strip()
            if indent == 10 and value.endswith(","):
                matrix_sequence.append(scalar(value.removesuffix(",")))
                continue
            if indent == 8 and value == "]":
                matrix_sequence = None
                continue
        job["__invalid__"] = True
    return job
