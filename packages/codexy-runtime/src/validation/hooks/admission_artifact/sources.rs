#[derive(Clone, Copy, Debug)]
pub(super) struct Source {
    pub(super) path: &'static str,
    pub(super) contents: &'static str,
}

macro_rules! source {
    ($path:literal) => {
        Source {
            path: $path,
            contents: include_str!(concat!("../../../../../../plugins/codexy/hooks/", $path)),
        }
    };
}

pub(super) const LAUNCHERS: &[Source] = &[
    source!("codexy-hook-runtime.sh"),
    source!("codexy-thread-delivery.sh"),
    source!("codexy-thread-delivery.cmd"),
    source!("codexy-repository-issue.sh"),
    source!("codexy-repository-issue.cmd"),
    source!("codexy-repository-pull-request.sh"),
    source!("codexy-repository-pull-request.cmd"),
    source!("codexy-repository-merge.sh"),
    source!("codexy-repository-merge.cmd"),
    source!("codexy-repository-github-command.sh"),
    source!("codexy-repository-github-command.cmd"),
    source!("codexy-destructive-command.sh"),
    source!("codexy-destructive-command.cmd"),
];

// This is the one compile-time source map. The runtime closure derives which
// pinned files the shipped entrypoint actually imports.
pub(super) const POLICY_SOURCES: &[Source] = &[
    source!("codexy-thread-delivery.py"),
    source!("codexy-repository-issue.py"),
    source!("codexy-repository-pull-request.py"),
    source!("codexy-repository-merge.py"),
    source!("codexy-repository-github-command.py"),
    source!("codexy-destructive-command.py"),
    source!("codexy_policy/__init__.py"),
    source!("codexy_policy/envelope.py"),
    source!("codexy_policy/thread_delivery.py"),
    source!("codexy_policy/repository_issue.py"),
    source!("codexy_policy/repository_pull_request.py"),
    source!("codexy_policy/repository_merge.py"),
    source!("codexy_policy/repository_github_command.py"),
    source!("codexy_policy/destructive_command.py"),
    source!("codexy_policy/body.py"),
    source!("codexy_policy/connector.py"),
    source!("codexy_policy/execution_context.py"),
    source!("codexy_policy/executable_identity.py"),
    source!("codexy_policy/filesystem_state.py"),
    source!("codexy_policy/git_command.py"),
    source!("codexy_policy/git_options.py"),
    source!("codexy_policy/git_runtime_config.py"),
    source!("codexy_policy/graphql.py"),
    source!("codexy_policy/graphql_parser.py"),
    source!("codexy_policy/github.py"),
    source!("codexy_policy/github_alias.py"),
    source!("codexy_policy/github_api.py"),
    source!("codexy_policy/github_target.py"),
    source!("codexy_policy/invocation.py"),
    source!("codexy_policy/invocation_wrappers.py"),
    source!("codexy_policy/merge.py"),
    source!("codexy_policy/pull_request.py"),
    source!("codexy_policy/repository.py"),
    source!("codexy_policy/repository_policy.py"),
    source!("codexy_policy/shell_destructive.py"),
    source!("codexy_policy/shell_destructive_opaque.py"),
    source!("codexy_policy/shell_destructive_policy.py"),
    source!("codexy_policy/shell_entry.py"),
    source!("codexy_policy/shell_evaluator.py"),
    source!("codexy_policy/shell_git.py"),
    source!("codexy_policy/shell_github.py"),
    source!("codexy_policy/shell_github_opaque.py"),
    source!("codexy_policy/shell_github_policy.py"),
    source!("codexy_policy/shell_opaque.py"),
    source!("codexy_policy/shell_builtins.py"),
    source!("codexy_policy/shell_context.py"),
    source!("codexy_policy/shell_groups.py"),
    source!("codexy_policy/shell_sequence.py"),
    source!("codexy_policy/titles.py"),
    source!("codexy_policy/wrappers.py"),
];
