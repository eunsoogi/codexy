use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

#[derive(Clone, Copy, Debug)]
struct Source {
    path: &'static str,
    contents: &'static str,
}

const LAUNCHERS: &[Source] = &[
    Source {
        path: "codexy-admission.sh",
        contents: include_str!("../../../plugins/codexy/hooks/codexy-admission.sh"),
    },
    Source {
        path: "codexy-admission.cmd",
        contents: include_str!("../../../plugins/codexy/hooks/codexy-admission.cmd"),
    },
];

// This is the one compile-time source map. `runtime_closure` derives the files
// the shipped Python entrypoint actually imports; it is intentionally not a
// second hand-maintained list of required files.
const POLICY_SOURCES: &[Source] = &[
    Source {
        path: "codexy-admission.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy-admission.py"),
    },
    Source {
        path: "codexy_policy/__init__.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/__init__.py"),
    },
    Source {
        path: "codexy_policy/admission.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/admission.py"),
    },
    Source {
        path: "codexy_policy/body.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/body.py"),
    },
    Source {
        path: "codexy_policy/execution_context.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/execution_context.py"),
    },
    Source {
        path: "codexy_policy/git_command.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/git_command.py"),
    },
    Source {
        path: "codexy_policy/github.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/github.py"),
    },
    Source {
        path: "codexy_policy/github_target.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/github_target.py"),
    },
    Source {
        path: "codexy_policy/invocation.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/invocation.py"),
    },
    Source {
        path: "codexy_policy/merge.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/merge.py"),
    },
    Source {
        path: "codexy_policy/pull_request.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/pull_request.py"),
    },
    Source {
        path: "codexy_policy/repository.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/repository.py"),
    },
    Source {
        path: "codexy_policy/shell.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/shell.py"),
    },
    Source {
        path: "codexy_policy/shell_context.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/shell_context.py"),
    },
    Source {
        path: "codexy_policy/titles.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/titles.py"),
    },
    Source {
        path: "codexy_policy/wrappers.py",
        contents: include_str!("../../../plugins/codexy/hooks/codexy_policy/wrappers.py"),
    },
];

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
