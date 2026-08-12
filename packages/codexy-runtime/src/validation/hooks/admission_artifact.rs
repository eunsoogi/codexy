use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

mod import_parser;
mod sources;
use import_parser::imports;
use sources::{LAUNCHERS, POLICY_SOURCES, Source};

pub(super) fn is_launcher(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(
            "codexy-thread-delivery.sh"
                | "codexy-thread-delivery.cmd"
                | "codexy-repository-issue.sh"
                | "codexy-repository-issue.cmd"
                | "codexy-repository-pull-request.sh"
                | "codexy-repository-pull-request.cmd"
                | "codexy-repository-merge.sh"
                | "codexy-repository-merge.cmd"
                | "codexy-repository-github-command.sh"
                | "codexy-repository-github-command.cmd"
                | "codexy-destructive-command.sh"
                | "codexy-destructive-command.cmd"
        )
    )
}

pub(super) fn check(plugin_root: &Path) -> Result<()> {
    let hooks = plugin_root.join("hooks");
    let sources = source_map();
    for source in LAUNCHERS {
        check_pinned(&hooks, source)?;
    }
    let closure = runtime_closure(&hooks, &sources)?;
    for path in closure {
        check_pinned(
            &hooks,
            sources
                .get(path.as_str())
                .expect("closure is manifest-backed"),
        )?;
    }
    Ok(())
}

fn source_map() -> BTreeMap<&'static str, &'static Source> {
    POLICY_SOURCES
        .iter()
        .map(|source| (source.path, source))
        .collect()
}

fn runtime_closure(hooks: &Path, sources: &BTreeMap<&str, &Source>) -> Result<BTreeSet<String>> {
    let mut closure = BTreeSet::new();
    for root in [
        "codexy-thread-delivery.py",
        "codexy-repository-issue.py",
        "codexy-repository-pull-request.py",
        "codexy-repository-merge.py",
        "codexy-repository-github-command.py",
        "codexy-destructive-command.py",
    ] {
        let concern = closure_from(root, hooks, sources)?;
        check_concern_boundary(root, &concern)?;
        closure.extend(concern);
    }
    Ok(closure)
}

fn closure_from(
    root: &str,
    hooks: &Path,
    sources: &BTreeMap<&str, &Source>,
) -> Result<BTreeSet<String>> {
    let mut closure = BTreeSet::new();
    visit(root, hooks, sources, &mut closure, &mut BTreeSet::new())?;
    Ok(closure)
}

fn check_concern_boundary(root: &str, closure: &BTreeSet<String>) -> Result<()> {
    let forbidden = match root {
        "codexy-destructive-command.py" => &[
            "codexy_policy/body.py",
            "codexy_policy/github.py",
            "codexy_policy/github_alias.py",
            "codexy_policy/github_api.py",
            "codexy_policy/github_target.py",
            "codexy_policy/graphql.py",
            "codexy_policy/graphql_parser.py",
            "codexy_policy/merge.py",
            "codexy_policy/pull_request.py",
            "codexy_policy/shell_github.py",
            "codexy_policy/shell_github_opaque.py",
            "codexy_policy/shell_github_policy.py",
            "codexy_policy/titles.py",
        ][..],
        "codexy-repository-github-command.py" => &[
            "codexy_policy/shell_destructive.py",
            "codexy_policy/shell_destructive_opaque.py",
            "codexy_policy/shell_destructive_policy.py",
        ][..],
        _ => return Ok(()),
    };
    if let Some(path) = forbidden.iter().find(|path| closure.contains(**path)) {
        bail!("packaged {root} import closure crosses concern boundary through {path}");
    }
    Ok(())
}

fn visit(
    path: &str,
    hooks: &Path,
    sources: &BTreeMap<&str, &Source>,
    closure: &mut BTreeSet<String>,
    visiting: &mut BTreeSet<String>,
) -> Result<()> {
    if closure.contains(path) {
        return Ok(());
    }
    if !visiting.insert(path.to_owned()) {
        bail!("packaged admission import cycle includes {path}");
    }
    let actual = read_regular(hooks, path)?;
    for imported in imports(path, &actual)? {
        if !sources.contains_key(imported.as_str()) {
            bail!("packaged admission import is unpinned: {imported}");
        }
        visit(&imported, hooks, sources, closure, visiting)?;
    }
    visiting.remove(path);
    closure.insert(path.to_owned());
    Ok(())
}

fn check_pinned(hooks: &Path, source: &Source) -> Result<()> {
    let actual = read_regular(hooks, source.path)?;
    if actual != source.contents {
        bail!(
            "packaged admission artifact bytes must match the validator-pinned source: {}",
            display_relative(&hooks.join(source.path))
        );
    }
    Ok(())
}

fn read_regular(hooks: &Path, relative: &str) -> Result<String> {
    let path = hooks.join(relative);
    let metadata = std::fs::symlink_metadata(&path).with_context(|| {
        format!(
            "reading packaged admission artifact {}",
            display_relative(&path)
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "packaged admission artifact must be a regular non-symlink file: {}",
            display_relative(&path)
        );
    }
    std::fs::read_to_string(&path).with_context(|| format!("reading {}", display_relative(&path)))
}
