"""Conservative structural checks for sensitive shell operations."""

from __future__ import annotations

import re
import shlex

from .git_command import normalize as normalize_git
from .git_options import normalize as normalize_git_options
from .github import forbidden as gh_forbidden
from .execution_context import ExecutionContext, at as context_at, git_config
from .invocation import resolve
from .repository import OWNED, UrlRewrite, github_identity, identity, repository_owned, rewrite_url
from .shell_context import changed_directory, flag
from .shell_groups import Command, GroupSyntaxError, Sequence, parse

OPAQUE = re.compile(r"\$\(|`|<<<?|\b(?:eval|if|for|while|until|case)\b")
SUBCOMMAND = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")
CONTROL = re.compile(r"<<<?|\b(?:if|for|while|until|case)\b")
POLICY_STATE = re.compile(r"(?:^|[;&|()\s])(?:git|gh|cd|source|\.|rm|export|unset|pushd|popd)(?=$|[;&|()\s])|\b(?:GIT_DIR|GH_REPO)\s*=")


def forbidden(command: str, cwd: str, gh_repo: str | None = None, depth: int = 0) -> bool:
    environment = (("GH_REPO", gh_repo),) if gh_repo is not None else ()
    return _forbidden(command, ExecutionContext(cwd, repository_owned(cwd), None, gh_repo, environment), depth)


def _forbidden(command: str, context: ExecutionContext, depth: int) -> bool:
    if depth > 3:
        return True
    lexical_command = command
    if OPAQUE.search(command):
        if context.cwd_owned is not False:
            return True
        try:
            opaque_tokens = shlex.split(command)
            if opaque_tokens and opaque_tokens[0].rsplit("/", 1)[-1].lower() == "eval":
                evaluated = opaque_tokens[1:]
                if evaluated[:1] == ["--"]:
                    evaluated = evaluated[1:]
                if _forbidden(" ".join(evaluated), context, depth + 1):
                    return True
            elif _explicit_owned(opaque_tokens) is True:
                return True
        except ValueError:
            return True
        for match in SUBCOMMAND.finditer(command):
            nested = match.group(1) if match.group(1) is not None else match.group(2)
            if _forbidden(nested, context, depth + 1):
                return True
        lexical_command = SUBCOMMAND.sub("__codexy_subcommand__", command)
        if CONTROL.search(command):
            return POLICY_STATE.search(command) is not None
    try:
        lexer = shlex.shlex(_separate_lines(lexical_command), posix=True, punctuation_chars=";&|(){}")
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return context.cwd_owned is not False
    try:
        sequence = parse(tokens)
    except GroupSyntaxError:
        return True
    return _sequence(sequence, context, depth)[0]


def _sequence(sequence: Sequence, context: ExecutionContext, depth: int) -> tuple[bool, ExecutionContext]:
    active = context
    for step in sequence.steps:
        if isinstance(step.node, Command):
            tokens = list(step.node.tokens)
            denied, resulting_context = _segment(tokens, active, depth)
            directory = changed_directory(tokens, active.cwd)
            if directory.opaque:
                return True, active
            resulting_context = context_at(resulting_context, directory.cwd)
        else:
            denied, nested_context = _sequence(step.node.body, active, depth + 1)
            resulting_context = active if step.node.kind == "subshell" else nested_context
        if denied:
            return True, active
        if step.following in {"", ";", "&&"} or (
            step.following == "||" and (active.cwd_owned is False or resulting_context.cwd_owned is not False)
        ):
            active = resulting_context
    return False, active


def _segment(tokens: list[str], context: ExecutionContext, depth: int) -> tuple[bool, ExecutionContext]:
    invocation = resolve(tokens, context, depth)
    if invocation is None:
        return True, context
    if invocation.script is not None:
        return (not invocation.script or _forbidden(invocation.script, invocation.context, depth + 1)), context
    if invocation.opaque:
        return True, context
    if invocation.executable is None:
        return False, invocation.context
    if invocation.executable in {".", "source"}:
        return True, context
    if invocation.executable == "git":
        return _git(invocation.arguments, invocation.context, depth), context
    if invocation.executable == "gh":
        gh_owned = github_identity(invocation.context.gh_repo) == OWNED if invocation.context.gh_repo is not None else None
        return gh_forbidden(invocation.arguments, invocation.context.cwd, invocation.context.cwd_owned, gh_owned), context
    if invocation.executable == "rm":
        return invocation.context.cwd_owned is not False and _rm(invocation.arguments), context
    return False, context


def _git(args: list[str], context: ExecutionContext, depth: int) -> bool:
    environment_config = git_config(context)
    if environment_config is None:
        return True
    invocation = normalize_git(args, context.cwd, context.cwd_owned, context.git_dir, _config_owned, environment_config)
    if invocation is None:
        return True
    if invocation.alias_command is not None:
        alias_context = ExecutionContext(
            invocation.cwd,
            invocation.cwd_owned,
            invocation.git_dir,
            context.gh_repo,
            context.environment,
            context.opaque_environment,
        )
        return not invocation.alias_command or _forbidden(invocation.alias_command, alias_context, depth + 1)
    if invocation.operation is None:
        return False
    target_owned = _explicit_owned(
        invocation.arguments, list(invocation.rewrites), invocation.operation == "push"
    )
    applies = target_owned is True or (target_owned is None and invocation.cwd_owned is not False)
    arguments = normalize_git_options(invocation.operation, invocation.arguments)
    if arguments is None:
        return applies
    if invocation.operation == "push":
        forced = any(arg in {"--force", "--force-with-lease", "--mirror"} or arg.startswith(("--force=", "--force-with-lease=", "--mirror=")) or (arg.startswith("-") and not arg.startswith("--") and "f" in arg[1:]) or arg.startswith("+") for arg in arguments)
        return applies and forced
    return applies and ((invocation.operation == "reset" and "--hard" in arguments) or (invocation.operation == "clean" and flag(arguments, "f", "--force")))


def _separate_lines(command: str) -> str:
    result, quote, escaped = [], None, False
    for char in command:
        if escaped:
            result.append(char)
            escaped = False
        elif char == "\\" and quote != "'":
            result.append(char)
            escaped = True
        elif char in {"'", '"'}:
            quote = None if quote == char else char if quote is None else quote
            result.append(char)
        else:
            result.append(";" if char == "\n" and quote is None else char)
    return "".join(result)


def _rm(args: list[str]) -> bool:
    targets = [arg for arg in args if not arg.startswith("-")]
    broad = {"/", "/*", "~", "$HOME", "${HOME}"}
    return flag(args, "r", "--recursive") and flag(args, "f", "--force") and any(target in broad or target.rstrip("/").endswith("/..") for target in targets)


def _explicit_owned(
    args: list[str], rewrites: list[UrlRewrite] | None = None, push: bool = False,
) -> bool | None:
    rewritten = [identity(rewrite_url(arg, rewrites or [], push)) for arg in args]
    identities = [item for item in rewritten if item is not None]
    return None if not identities else OWNED in identities


def _config_owned(config: str) -> bool:
    return "=" in config and identity(config.split("=", 1)[1]) == OWNED
