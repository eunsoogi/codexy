"""Conservative structural checks for sensitive shell operations."""

from __future__ import annotations

import re
import shlex

from .git_command import normalize as normalize_git
from .github import forbidden as gh_forbidden
from .invocation import resolve
from .repository import OWNED, github_identity, identity, repository_owned
from .shell_context import changed_directory, flag

OPS = {";", "&&", "||", "|", "&"}
OPAQUE = re.compile(r"\$\(|`|\$\{|<<<?|\b(?:eval|if|for|while|until|case)\b")
SUBCOMMAND = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")


def forbidden(command: str, cwd: str, depth: int = 0) -> bool:
    cwd_owned = repository_owned(cwd)
    if depth > 3:
        return True
    if OPAQUE.search(command):
        if cwd_owned is not False:
            return True
        try:
            opaque_tokens = shlex.split(command)
            if opaque_tokens and opaque_tokens[0].rsplit("/", 1)[-1].lower() == "eval":
                evaluated = opaque_tokens[1:]
                if evaluated[:1] == ["--"]:
                    evaluated = evaluated[1:]
                if forbidden(" ".join(evaluated), cwd, depth + 1):
                    return True
            elif _explicit_owned(opaque_tokens) is True:
                return True
        except ValueError:
            return True
        for match in SUBCOMMAND.finditer(command):
            nested = match.group(1) if match.group(1) is not None else match.group(2)
            if forbidden(nested, cwd, depth + 1):
                return True
        if re.search(r"\$\{|<<<?|\b(?:if|for|while|until|case)\b", command):
            return True
    try:
        lexer = shlex.shlex(command.replace("\n", ";"), posix=True, punctuation_chars=";&|(){}")
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return cwd_owned is not False
    if any(token in {"(", ")", "{", "}"} for token in tokens):
        return cwd_owned is not False
    segments, current = [], []
    for token in tokens:
        if token in OPS:
            if current:
                segments.append((current, token))
                current = []
        else:
            current.append(token)
    if current:
        segments.append((current, ""))
    active_cwd = cwd
    for index, (segment, following) in enumerate(segments):
        if _segment(segment, active_cwd, depth):
            return True
        changed_cwd = changed_directory(segment, active_cwd)
        if following in {";", "&&"} or (following == "||" and (repository_owned(active_cwd) is False or repository_owned(changed_cwd) is not False)):
            active_cwd = changed_cwd
        if following == "|" and index + 1 < len(segments):
            next_invocation = resolve(segments[index + 1][0], active_cwd, depth)
            if next_invocation is None or next_invocation.opaque:
                return repository_owned(active_cwd) is not False
    return False


def _segment(tokens: list[str], cwd: str, depth: int) -> bool:
    invocation = resolve(tokens, cwd, depth)
    if invocation is None:
        return True
    if invocation.script is not None:
        return not invocation.script or forbidden(invocation.script, invocation.cwd, depth + 1)
    if invocation.opaque:
        return invocation.cwd_owned is not False
    if invocation.executable is None:
        return False
    if invocation.executable == "git":
        return _git(invocation.arguments, invocation.cwd, invocation.cwd_owned, invocation.git_dir)
    if invocation.executable == "gh":
        gh_owned = github_identity(invocation.gh_repo) == OWNED if invocation.gh_repo is not None else None
        return gh_forbidden(invocation.arguments, invocation.cwd_owned, gh_owned)
    if invocation.executable == "rm":
        return invocation.cwd_owned is not False and _rm(invocation.arguments)
    return False


def _git(args: list[str], cwd: str, cwd_owned: bool | None, git_dir: str | None) -> bool:
    invocation = normalize_git(args, cwd, cwd_owned, git_dir, _config_owned)
    if invocation is None:
        return True
    if invocation.alias_command is not None:
        return not invocation.alias_command or forbidden(invocation.alias_command, cwd, 1)
    if invocation.operation is None:
        return False
    target_owned = _explicit_owned(invocation.arguments)
    applies = target_owned is True or (target_owned is None and invocation.cwd_owned is not False)
    if invocation.operation == "push":
        forced = any(arg in {"--force", "--force-with-lease", "--mirror"} or arg.startswith(("--force=", "--force-with-lease=", "--mirror=")) or (arg.startswith("-") and not arg.startswith("--") and "f" in arg[1:]) or arg.startswith("+") for arg in invocation.arguments)
        return applies and forced
    return applies and ((invocation.operation == "reset" and "--hard" in invocation.arguments) or (invocation.operation == "clean" and flag(invocation.arguments, "f", "--force")))


def _rm(args: list[str]) -> bool:
    targets = [arg for arg in args if not arg.startswith("-")]
    broad = {"/", "/*", "~", "$HOME", "${HOME}"}
    return flag(args, "r", "--recursive") and flag(args, "f", "--force") and any(target in broad or target.rstrip("/").endswith("/..") for target in targets)


def _explicit_owned(args: list[str]) -> bool | None:
    identities = [identity(arg) for arg in args if identity(arg) is not None]
    return None if not identities else OWNED in identities


def _config_owned(config: str) -> bool:
    return "=" in config and identity(config.split("=", 1)[1]) == OWNED
