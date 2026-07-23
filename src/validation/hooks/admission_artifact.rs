use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

mod sources;
use sources::{LAUNCHERS, POLICY_SOURCES, Source};

pub(super) fn is_launcher(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("codexy-admission.sh" | "codexy-admission.cmd")
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
    let mut visiting = BTreeSet::new();
    visit(
        "codexy-admission.py",
        hooks,
        sources,
        &mut closure,
        &mut visiting,
    )?;
    Ok(closure)
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

fn imports(path: &str, source: &str) -> Result<Vec<String>> {
    if source.lines().map(str::trim_start).any(|line| {
        line.starts_with("importlib.")
            || line.starts_with("__import__(")
            || line.starts_with("exec(")
    }) {
        bail!("packaged admission runtime rejects dynamic imports: {path}");
    }
    let mut result = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        let Some((prefix, _)) = line.split_once(" import ") else {
            continue;
        };
        let Some(module) = prefix.strip_prefix("from ") else {
            continue;
        };
        if let Some(module) = module.strip_prefix('.') {
            if module.is_empty()
                || !module
                    .chars()
                    .all(|value| value.is_ascii_alphanumeric() || value == '_')
            {
                bail!("packaged admission runtime rejects ambiguous relative import in {path}");
            }
            result.push(format!("codexy_policy/{module}.py"));
        } else if let Some(module) = module.strip_prefix("codexy_policy.") {
            if !module
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || value == '.')
            {
                bail!("packaged admission runtime rejects ambiguous policy import in {path}");
            }
            result.push(format!("codexy_policy/{}.py", module.replace('.', "/")));
        }
    }
    if path.starts_with("codexy_policy/") && path != "codexy_policy/__init__.py" {
        result.push("codexy_policy/__init__.py".to_owned());
    }
    Ok(result)
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
