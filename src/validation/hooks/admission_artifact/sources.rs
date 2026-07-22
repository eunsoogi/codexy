#[derive(Clone, Copy, Debug)]
pub(super) struct Source {
    pub(super) path: &'static str,
    pub(super) contents: &'static str,
}

macro_rules! source {
    ($path:literal) => {
        Source {
            path: $path,
            contents: include_str!(concat!("../../../../plugins/codexy/hooks/", $path)),
        }
    };
}

pub(super) const LAUNCHERS: &[Source] = &[
    Source {
        path: "codexy-admission.sh",
        contents: include_str!("../../../../plugins/codexy/hooks/codexy-admission.sh"),
    },
    Source {
        path: "codexy-admission.cmd",
        contents: include_str!("../../../../plugins/codexy/hooks/codexy-admission.cmd"),
    },
];

// This is the one compile-time source map. The runtime closure derives which
// pinned files the shipped entrypoint actually imports.
pub(super) const POLICY_SOURCES: &[Source] = &[
    source!("codexy-admission.py"),
    source!("codexy_policy/__init__.py"),
    source!("codexy_policy/admission.py"),
    source!("codexy_policy/body.py"),
    source!("codexy_policy/execution_context.py"),
    source!("codexy_policy/git_command.py"),
    source!("codexy_policy/git_options.py"),
    source!("codexy_policy/git_runtime_config.py"),
    source!("codexy_policy/github.py"),
    source!("codexy_policy/github_alias.py"),
    source!("codexy_policy/github_api.py"),
    source!("codexy_policy/github_target.py"),
    source!("codexy_policy/invocation.py"),
    source!("codexy_policy/merge.py"),
    source!("codexy_policy/pull_request.py"),
    source!("codexy_policy/repository.py"),
    source!("codexy_policy/shell.py"),
    source!("codexy_policy/shell_context.py"),
    source!("codexy_policy/shell_groups.py"),
    source!("codexy_policy/titles.py"),
    source!("codexy_policy/wrappers.py"),
];
