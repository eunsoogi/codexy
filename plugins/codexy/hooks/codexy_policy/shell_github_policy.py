"""Repository GitHub-command concern policy."""

from .execution_context import CommandEffect, ExecutionContext, remote_url
from .github import forbidden as gh_forbidden
from .github_alias import expand as expand_gh_alias
from .repository import OWNED, github_identity
from .shell_git import evaluate as evaluate_git
from .shell_github_opaque import owns as github_opaque


class GithubPolicy:
    owns_opaque = staticmethod(github_opaque)

    @staticmethod
    def opaque_invocation(tokens: list[str]) -> bool:
        return github_opaque(" ".join(tokens))

    def command(self, invocation, outer: ExecutionContext, depth: int):
        if invocation.executable == "git":
            _, remote, alias = evaluate_git(invocation.arguments, invocation.context)
            if alias is not None:
                from .shell_evaluator import evaluate
                denied = evaluate(alias, invocation.context, depth + 1, self)
                return denied, CommandEffect(outer)
            if remote is None:
                return False, CommandEffect(outer)
            if invocation.context.cwd != outer.cwd or invocation.context.git_dir != outer.git_dir:
                return True, CommandEffect(None)
            return False, CommandEffect(remote_url(outer, *remote))
        if invocation.executable == "gh":
            gh_owned = (
                github_identity(invocation.context.gh_repo) == OWNED
                if invocation.context.gh_repo is not None else None
            )
            arguments = expand_gh_alias(invocation.arguments)
            denied = arguments is None or gh_forbidden(
                arguments, invocation.context.cwd,
                invocation.context.cwd_owned, gh_owned,
            )
            return denied, CommandEffect(outer)
        if invocation.executable in {"hash", "rm"}:
            return False, CommandEffect(outer)
        return None


POLICY = GithubPolicy()
