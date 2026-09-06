#[derive(Clone, Copy, Debug)]
pub(super) struct Source {
    pub(super) path: &'static str,
    pub(super) contents: &'static str,
}

macro_rules! source {
    ($path:literal) => {
        Source {
            path: $path,
            contents: include_str!(concat!(
                "../../../../../../plugins/codexy-github/hooks/",
                $path
            )),
        }
    };
}

pub(super) const LAUNCHERS: &[Source] = &[
    source!("codexy-hook-runtime.sh"),
    source!("codexy-destructive-command.sh"),
    source!("codexy-destructive-command.cmd"),
];

// This is the one compile-time source map. The runtime closure derives which
// pinned files the shipped entrypoint actually imports.
pub(super) const POLICY_SOURCES: &[Source] = &[
    source!("codexy-destructive-command.py"),
    source!("codexy_policy/__init__.py"),
    source!("codexy_policy/envelope.py"),
    source!("codexy_policy/destructive_command.py"),
    source!("codexy_policy/execution_context.py"),
    source!("codexy_policy/execution_context_types.py"),
    source!("codexy_policy/execution_filesystem.py"),
    source!("codexy_policy/executable_digest.py"),
    source!("codexy_policy/executable_identity.py"),
    source!("codexy_policy/filesystem_state.py"),
    source!("codexy_policy/git_command.py"),
    source!("codexy_policy/git_command_options.py"),
    source!("codexy_policy/git_options.py"),
    source!("codexy_policy/invocation.py"),
    source!("codexy_policy/invocation_options.py"),
    source!("codexy_policy/invocation_wrappers.py"),
    source!("codexy_policy/repository.py"),
    source!("codexy_policy/repository_aliases.py"),
    source!("codexy_policy/shell_destructive.py"),
    source!("codexy_policy/shell_destructive_opaque.py"),
    source!("codexy_policy/shell_destructive_policy.py"),
    source!("codexy_policy/shell_entry.py"),
    source!("codexy_policy/shell_evaluator.py"),
    source!("codexy_policy/shell_git.py"),
    source!("codexy_policy/shell_opaque.py"),
    source!("codexy_policy/shell_reflog.py"),
    source!("codexy_policy/shell_segments.py"),
    source!("codexy_policy/shell_builtins.py"),
    source!("codexy_policy/policy_diagnostics.py"),
    source!("codexy_policy/shell_context.py"),
    source!("codexy_policy/shell_groups.py"),
    source!("codexy_policy/shell_sequence.py"),
];
