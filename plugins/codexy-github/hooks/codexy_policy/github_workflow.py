"""Closed GitHub workflow operations admitted by the repository policy."""

from __future__ import annotations

import re

from .github_mutation import read_command
from .repository_identity import github_identity

_WORKFLOW = "plugin-version-bump.yml"
_VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+\Z")
_CACHE_SAFE_API_OPTIONS = frozenset(
    {
        "--cache",
        "--include",
        "-i",
        "--jq",
        "--paginate",
        "--preview",
        "--silent",
        "--slurp",
        "--template",
        "--verbose",
    }
)


def read_or_admit(
    args: list[str],
    selected_target: tuple[list[str], bool, str | None],
    owned_identity: tuple[str, str, str] | None,
) -> bool:
    """Admit a read command or one exact, owned workflow operation."""
    return read_only(args) or workflow(selected_target, owned_identity)


def read_only(args: list[str]) -> bool:
    """Recognize read-only commands even when a data argument is opaque."""
    if read_command(args):
        return True
    if args[:1] != ["api"]:
        return False
    method = "GET"
    index = 1
    while index < len(args):
        token = args[index]
        if token in {"-X", "--method"}:
            if index + 1 >= len(args) or method != "GET":
                return False
            method, index = args[index + 1].upper(), index + 2
        elif token.startswith(("-X=", "--method=")):
            if method != "GET":
                return False
            method, index = token.split("=", 1)[1].upper(), index + 1
        elif token in {"-f", "--field", "--raw-field", "--input"}:
            return False
        elif token.startswith(("-f=", "--field=", "--raw-field=", "--input=")):
            return False
        elif token in {"-H", "--header"} or token.startswith(("-H=", "--header=")):
            return False
        elif token in _CACHE_SAFE_API_OPTIONS:
            index += 2 if token in {"--cache", "--jq", "--preview", "--template"} else 1
            continue
        elif token.startswith(("--cache=", "--jq=", "--preview=", "--template=")):
            index += 1
            continue
        elif token.startswith("-"):
            return False
        index += 1
    return method in {"GET", "HEAD"}


def workflow(
    selected_target: tuple[list[str], bool, str | None],
    owned_identity: tuple[str, str, str] | None,
) -> bool:
    filtered, _, repository = selected_target
    if repository is None or github_identity(repository) != owned_identity:
        return False
    operation = tuple(filtered[:2])
    if operation == ("workflow", "run"):
        return _dispatch(filtered[2:])
    if operation == ("run", "rerun"):
        return _rerun(filtered[2:])
    return False


def _dispatch(args: list[str]) -> bool:
    if len(args) < 3 or args[0] != _WORKFLOW:
        return False
    fields: dict[str, str] = {}
    ref = None
    index = 1
    while index < len(args):
        token = args[index]
        if token == "--ref" and ref is None and index + 1 < len(args):
            ref, index = args[index + 1], index + 2
        elif token.startswith("--ref=") and ref is None:
            ref, index = token.split("=", 1)[1], index + 1
        elif token in {"-f", "--field"} and index + 1 < len(args):
            if not _field(fields, args[index + 1]):
                return False
            index += 2
        elif token.startswith(("-f=", "--field=")):
            if not _field(fields, token.split("=", 1)[1]):
                return False
            index += 1
        else:
            return False
    return (
        (ref is None or ref == "main")
        and fields.keys() == {"version", "issue"}
        and bool(_VERSION.fullmatch(fields["version"]) and _positive(fields["issue"]))
    )


def _rerun(args: list[str]) -> bool:
    return len(args) == 1 and _positive(args[0])


def _field(fields: dict[str, str], value: str) -> bool:
    name, separator, content = value.partition("=")
    if separator != "=" or name in fields or name not in {"version", "issue"}:
        return False
    fields[name] = content
    return True


def _positive(value: str) -> bool:
    return value.isascii() and value.isdigit() and int(value) > 0
