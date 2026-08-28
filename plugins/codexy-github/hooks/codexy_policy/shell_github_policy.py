"""Repository GitHub-command concern policy."""

from .execution_context import CommandEffect, ExecutionContext, remote_url
from .github import forbidden as gh_forbidden
from .github_alias import expand as expand_gh_alias
from .repository import github_identity
from .shell_git import evaluate as evaluate_git
from .shell_github_opaque import owns as github_opaque, owns_invocation


class GithubPolicy:
    redirection_executables = frozenset({"gh"})
    owns_opaque = staticmethod(github_opaque)

    @staticmethod
    def opaque_invocation(invocation) -> bool:
        return owns_invocation(invocation)

    def command(self, invocation, outer: ExecutionContext, depth: int):
        if invocation.executable == "git":
            _, remote, alias = evaluate_git(invocation.arguments, invocation.context)
            if alias is not None:
                from .shell_evaluator import evaluate

                denied = evaluate(alias.command, alias.context, depth + 1, self)
                return denied, CommandEffect(outer)
            if remote is None:
                return False, CommandEffect(outer)
            if (
                invocation.context.cwd != outer.cwd
                or invocation.context.git_dir != outer.git_dir
            ):
                return True, CommandEffect(None)
            return False, CommandEffect(remote_url(outer, *remote))
        if invocation.executable == "gh":
            gh_owned = (
                github_identity(invocation.context.gh_repo)
                == invocation.context.policy_identity
                if invocation.context.gh_repo is not None
                else None
            )
            arguments = expand_gh_alias(invocation.arguments)
            denied = arguments is None or gh_forbidden(
                arguments,
                invocation.context.cwd,
                invocation.context.cwd_owned,
                gh_owned,
                invocation.context.policy_identity,
                invocation.context.policy_status,
                policy_bound=True,
            )
            return denied, CommandEffect(outer)
        if invocation.executable in {"hash", "rm"}:
            return False, CommandEffect(outer)
        return None


POLICY = GithubPolicy()
