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
        || token_coverage::moved_token_coverage(&removed, &extracted) >= 2)
}

fn markdown_extraction(root: &Path, base_ref: &str, path: &Path, current: &str) -> Result<String> {
    let Some(directory) = facade_directory(path) else {
        return Ok(String::new());
    };
    let parent = path.parent().unwrap_or(Path::new(""));
    let mut modules = std::collections::BTreeSet::new();
    for target in current.lines().filter_map(markdown_link_target) {
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
    if modules.len() < 2 {
        return Ok(String::new());
    }
    extracted_new_lines(root, base_ref, modules)
}

fn semantic_markdown_component(component: &str) -> bool {
    if component.is_empty() || matches!(component, "." | "..") {
        return false;
    }
    if !component.bytes().any(|byte| byte.is_ascii_digit()) {
        return true;
    }
    let Some(rule) = component.strip_prefix('c') else {
        return false;
    };
    let digits = rule.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0
        && rule
            .get(digits..)
            .is_some_and(|suffix| suffix.starts_with('-'))
}

fn markdown_link_target(line: &str) -> Option<&str> {
    line.trim()
        .split_once("](")
        .and_then(|(_, target)| target.strip_suffix(')'))
        .filter(|target| !target.contains('#'))
}

fn rust_module_extraction(
    root: &Path,
    base_ref: &str,
    path: &Path,
    current: &str,
) -> Result<String> {
    let facade_directory = facade_directory(path);
    let mut modules = std::collections::BTreeSet::new();
    let mut explicit_path = None;
    for line in current.lines() {
        let line = line.trim();
        if let Some(path) = line
            .strip_prefix("#[path = \\")
            .and_then(|path| path.strip_suffix("\"]"))
        {
            explicit_path = Some(path);
            continue;
        }
        let declaration = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        let Some(module) = declaration
            .strip_prefix("mod ")
            .and_then(|name| name.strip_suffix(';'))
        else {
            continue;
        };
        if module.bytes().any(|byte| byte.is_ascii_digit()) {
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
        if root.join(&module_path).is_file() {
            modules.insert(module_path);
        }
    }
    extracted_new_lines(root, base_ref, modules)
}

fn facade_directory(path: &Path) -> Option<PathBuf> {
    path.file_stem()
        .map(|stem| path.parent().unwrap_or(Path::new("")).join(stem))
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
