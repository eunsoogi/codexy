use std::path::{Path, PathBuf};

use anyhow::Result;

use super::{read_base_text, token_coverage};

pub(super) fn has_new_module_boundary(
    root: &Path,
    base_ref: &str,
    path: &Path,
    base: &str,
    current: &str,
) -> Result<bool> {
    let current_lines = current.lines().collect::<std::collections::HashSet<_>>();
    let removed = base
        .lines()
        .filter(|line| !line.trim().is_empty() && !current_lines.contains(line))
        .collect::<Vec<_>>();
    if removed.is_empty() {
        return Ok(false);
    }
    let extracted = match path.extension().and_then(|extension| extension.to_str()) {
        Some("md") => markdown_extraction(root, base_ref, path, current)?,
        _ => rust_module_extraction(root, base_ref, path, current)?,
    };
    let removed = removed.join("\n");
    Ok(token_coverage::without_whitespace(&extracted)
        .contains(&token_coverage::without_whitespace(&removed))
        || token_coverage::moved_token_coverage(&removed, &extracted) >= 2
            && token_coverage::nonempty_line_count(&extracted).saturating_mul(4)
                >= token_coverage::nonempty_line_count(&removed).saturating_mul(3))
}

fn markdown_extraction(root: &Path, base_ref: &str, path: &Path, current: &str) -> Result<String> {
    let Some(directory) = markdown_facade_directory(path) else {
        return Ok(String::new());
    };
    let parent = path.parent().unwrap_or(Path::new(""));
    let mut modules = std::collections::BTreeSet::new();
    for target in current.lines().flat_map(markdown_link_targets) {
        let module = parent.join(target);
        if target
            .split('/')
            .any(|part| !semantic_markdown_component(part))
            || module.extension().and_then(|extension| extension.to_str()) != Some("md")
            || !module.starts_with(&directory)
            || !root.join(&module).is_file()
        {
            continue;
        }
        modules.insert(module);
    }
    if modules.is_empty() {
        return Ok(String::new());
    }
    extracted_new_lines(root, base_ref, modules)
}

fn semantic_markdown_component(component: &str) -> bool {
    if component.is_empty() || matches!(component, "." | "..") {
        return false;
    }
    !mechanical_numbered_component(component)
}

fn mechanical_numbered_component(component: &str) -> bool {
    let stem = component.strip_suffix(".md").unwrap_or(component);
    ["shard", "part", "chunk"].iter().any(|prefix| {
        stem.strip_prefix(prefix)
            .map(|suffix| suffix.trim_start_matches(['-', '_']))
            .is_some_and(|suffix| {
                let digits = suffix.strip_prefix('v').unwrap_or(suffix);
                !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
            })
    })
}

fn markdown_link_targets(line: &str) -> impl Iterator<Item = &str> {
    line.trim()
        .split("](")
        .skip(1)
        .filter_map(|target| target.split_once(')').map(|(target, _)| target))
        .filter_map(|target| target.split('#').next())
        .filter(|target| !target.is_empty())
}

fn rust_module_extraction(
    root: &Path,
    base_ref: &str,
    path: &Path,
    current: &str,
) -> Result<String> {
    let mut modules = std::collections::BTreeSet::new();
    collect_rust_modules(root, path, current, &mut modules);
    extracted_new_lines(root, base_ref, modules)
}

fn collect_rust_modules(
    root: &Path,
    path: &Path,
    current: &str,
    modules: &mut std::collections::BTreeSet<PathBuf>,
) {
    let facade_directory = facade_directory(path);
    let mut explicit_path = None;
    for line in current.lines() {
        let mut line = line.trim();
        if let Some((path, remainder)) = rust_path_attribute(line) {
            explicit_path = Some(path);
            if remainder.is_empty() {
                continue;
            }
            line = remainder;
        }
        let declaration = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        let Some(module) = declaration
            .strip_prefix("mod ")
            .and_then(|name| name.strip_suffix(';'))
        else {
            if !line.is_empty() && !line.starts_with("#[") && !line.starts_with("//") {
                explicit_path = None;
            }
            continue;
        };
        if mechanical_numbered_component(module) {
            explicit_path = None;
            continue;
        }
        let module_path = if let Some(explicit_path) = explicit_path.take() {
            path.parent().unwrap_or(Path::new("")).join(explicit_path)
        } else {
            let sibling = path
                .parent()
                .unwrap_or(Path::new(""))
                .join(format!("{module}.rs"));
            facade_directory
                .as_ref()
                .map(|directory| directory.join(format!("{module}.rs")))
                .filter(|candidate| root.join(candidate).is_file())
                .unwrap_or(sibling)
        };
        if root.join(&module_path).is_file() && modules.insert(module_path.clone()) {
            let module = std::fs::read_to_string(root.join(&module_path)).unwrap_or_default();
            collect_rust_modules(root, &module_path, &module, modules);
        }
    }
}

fn rust_path_attribute(line: &str) -> Option<(&str, &str)> {
    let attribute = line.strip_prefix("#[path")?;
    let closing = attribute.find(']')?;
    let path = attribute[..closing]
        .trim()
        .strip_prefix('=')?
        .trim()
        .strip_prefix('"')?
        .strip_suffix('"')?;
    Some((path, attribute[closing + 1..].trim()))
}

fn facade_directory(path: &Path) -> Option<PathBuf> {
    path.file_stem()
        .map(|stem| path.parent().unwrap_or(Path::new("")).join(stem))
}

fn markdown_facade_directory(path: &Path) -> Option<PathBuf> {
    if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
        return path.parent().map(|parent| parent.join("references"));
    }
    facade_directory(path)
}

fn extracted_new_lines(
    root: &Path,
    base_ref: &str,
    modules: std::collections::BTreeSet<PathBuf>,
) -> Result<String> {
    let mut extracted = String::new();
    for module_path in modules {
        let current_module = std::fs::read_to_string(root.join(&module_path)).unwrap_or_default();
        let base_module = read_base_text(root, base_ref, &module_path)?.unwrap_or_default();
        let base_module_lines = base_module
            .lines()
            .collect::<std::collections::HashSet<_>>();
        extracted.push_str(
            &current_module
                .lines()
                .filter(|line| !line.trim().is_empty() && !base_module_lines.contains(line))
                .map(|line| format!("{line}\n"))
                .collect::<String>(),
        );
    }
    Ok(extracted)
}
