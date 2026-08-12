"""Destructive shell/Git concern policy."""

from .execution_context import CommandEffect, ExecutionContext, remote_url
from .shell_builtins import hash_path_alias, rm_forbidden
from .shell_destructive_opaque import owns as destructive_opaque
from .shell_git import evaluate as evaluate_git


class DestructivePolicy:
    owns_opaque = staticmethod(destructive_opaque)

    @staticmethod
    def opaque_invocation(tokens: list[str], context: ExecutionContext) -> bool:
        del tokens, context
        return True

    def command(self, invocation, outer: ExecutionContext, depth: int):
        if invocation.executable == "hash" and hash_path_alias(invocation.arguments):
            return True, CommandEffect(None)
        if invocation.executable == "git":
            denied, remote, alias = evaluate_git(invocation.arguments, invocation.context)
            if alias is not None:
                from .shell_evaluator import evaluate
                denied = evaluate(alias.command, alias.context, depth + 1, self)
            if remote is None:
                return denied, CommandEffect(outer)
            if invocation.context.cwd != outer.cwd or invocation.context.git_dir != outer.git_dir:
                return True, CommandEffect(None)
            return denied, CommandEffect(remote_url(outer, *remote))
        if invocation.executable == "gh":
            return False, CommandEffect(outer)
        if invocation.executable == "rm":
            denied = invocation.context.cwd_owned is not False and rm_forbidden(invocation.arguments)
            return denied, CommandEffect(outer)
        return None


POLICY = DestructivePolicy()
