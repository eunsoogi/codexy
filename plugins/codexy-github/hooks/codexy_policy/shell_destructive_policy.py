"""Destructive shell/Git concern policy."""

from .execution_context import CommandEffect, ExecutionContext, remote_url
from .shell_builtins import find_forbidden, hash_path_alias, rm_forbidden
from .shell_destructive_opaque import owns as destructive_opaque, owns_invocation
from .shell_git import evaluate as evaluate_git


class DestructivePolicy:
    redirection_executables = frozenset({"gh", "git"})
    owns_opaque = staticmethod(destructive_opaque)

    @staticmethod
    def opaque_invocation(invocation) -> bool:
        return owns_invocation(invocation)

    def command(self, invocation, outer: ExecutionContext, depth: int):
        if invocation.executable == "hash" and hash_path_alias(invocation.arguments):
            return True, CommandEffect(None)
        if invocation.executable == "git":
            denied, remote, alias = evaluate_git(
                invocation.arguments, invocation.context
            )
            if alias is not None:
                from .shell_evaluator import evaluate

                denied = evaluate(alias.command, alias.context, depth + 1, self)
            if remote is None:
                return denied, CommandEffect(outer)
            if (
                invocation.context.cwd != outer.cwd
                or invocation.context.git_dir != outer.git_dir
            ):
                return True, CommandEffect(None)
            return denied, CommandEffect(remote_url(outer, *remote))
        if invocation.executable == "gh":
            return False, CommandEffect(outer)
        if invocation.executable == "rm":
            denied = invocation.context.cwd_owned is not False and (
                invocation.opaque
                or rm_forbidden(invocation.arguments, invocation.context.cwd)
            )
            return denied, CommandEffect(outer)
        if invocation.executable == "find":
            denied = invocation.context.cwd_owned is not False and (
                invocation.opaque
                or find_forbidden(invocation.arguments, invocation.context.cwd)
            )
            return denied, CommandEffect(outer)
        return None


POLICY = DestructivePolicy()
