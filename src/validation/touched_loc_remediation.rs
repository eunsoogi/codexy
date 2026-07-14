use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

mod rust_module;

pub(super) fn formatting_only_error(
    root: &Path,
    change_base_ref: &str,
    baseline_ref: &str,
    path: &Path,
    current_lines: usize,
    loc_limit: usize,
) -> Result<Option<String>> {
    let base_path = base_path(root, baseline_ref, path)?;
    let Some(base_text) = read_base_text(root, baseline_ref, &base_path)? else {
        return Ok(None);
    };
    if base_text.lines().count() <= loc_limit || current_lines > loc_limit {
        return Ok(None);
    }
    let current_text = std::fs::read_to_string(root.join(path))
        .with_context(|| format!("reading touched file {}", path.display()))?;
    let same_nonempty_lines = nonempty_line_count(&base_text) == nonempty_line_count(&current_text);
    let formatting_only = without_whitespace(&base_text) == without_whitespace(&current_text);
    let concealed_collapse = !same_nonempty_lines
        && !has_new_module_boundary(root, baseline_ref, path, &base_text, &current_text)?
        && !has_test_target_split(
            root,
            change_base_ref,
            baseline_ref,
            path,
            &base_text,
            &current_text,
        )?
        && !removed_lines_are_duplicates(&base_text, &current_text);
    if !formatting_only && !concealed_collapse {
        return Ok(None);
    }
    let reduction = if same_nonempty_lines {
        "blank-line deletion or other whitespace-only compression"
    } else {
        "multiline collapse or other formatting-only compression"
    };
    Ok(Some(format!(
        "{} reached the {loc_limit}-line LOC target through {reduction}; use coherent structural refactoring instead",
        path.display()
    )))
}

fn has_test_target_split(
    root: &Path,
    change_base_ref: &str,
    baseline_ref: &str,
    path: &Path,
    base: &str,
    current: &str,
) -> Result<bool> {
    if !path.starts_with("tests/") {
        return Ok(false);
    }
    let mut removed = std::collections::HashMap::<String, usize>::new();
    for line in base.lines().map(str::trim).filter(|line| !line.is_empty()) {
        *removed.entry(line.to_owned()).or_default() += 1;
    }
    for line in current
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        removed
            .entry(line.to_owned())
            .and_modify(|count| *count = count.saturating_sub(1));
    }
    let mut added = std::collections::HashMap::<String, usize>::new();
    for candidate in super::touched_loc::changed_files(root, change_base_ref)? {
        if candidate == path
            || candidate.parent() != path.parent()
            || candidate
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("rs")
            || read_base_text(root, baseline_ref, &candidate)?.is_some()
        {
            continue;
        }
        let text = std::fs::read_to_string(root.join(candidate)).unwrap_or_default();
        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            *added.entry(line.to_owned()).or_default() += 1;
        }
    }
    let moved = removed
        .iter()
        .map(|(line, count)| count.min(added.get(line).unwrap_or(&0)))
        .sum::<usize>();
    let required = nonempty_line_count(base).saturating_sub(nonempty_line_count(current));
    Ok(required > 0 && moved.saturating_mul(4) >= required.saturating_mul(3))
}

fn has_new_module_boundary(
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
        .collect::<String>();
    let removed = removed.split_whitespace().collect::<String>();
    if removed.is_empty() {
        return Ok(false);
    }
    for module_path in rust_module::declared_paths(root, path, current) {
        let current_module = std::fs::read_to_string(root.join(&module_path)).unwrap_or_default();
        let base_module = read_base_text(root, base_ref, &module_path)?.unwrap_or_default();
        let base_module_lines = base_module
            .lines()
            .collect::<std::collections::HashSet<_>>();
        let added = current_module
            .lines()
            .filter(|line| !line.trim().is_empty() && !base_module_lines.contains(line))
            .collect::<String>();
        if added
            .split_whitespace()
            .collect::<String>()
            .contains(&removed)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn removed_lines_are_duplicates(base: &str, current: &str) -> bool {
    let mut base_counts = std::collections::HashMap::new();
    let mut current_counts = std::collections::HashMap::new();
    for line in base.lines().filter(|line| !line.trim().is_empty()) {
        *base_counts.entry(line).or_insert(0usize) += 1;
    }
    for line in current.lines().filter(|line| !line.trim().is_empty()) {
        *current_counts.entry(line).or_insert(0usize) += 1;
    }
    let required_reduction = nonempty_line_count(base).saturating_sub(nonempty_line_count(current));
    let duplicate_reduction = base_counts
        .iter()
        .map(|(line, count)| {
            current_counts
                .get(line)
                .filter(|current| **current > 0)
                .map_or(0, |current| count.saturating_sub(*current))
        })
        .sum::<usize>();
    required_reduction > 0 && duplicate_reduction >= required_reduction
}

fn base_path(root: &Path, base_ref: &str, path: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["diff", "--name-status", "--find-renames", base_ref, "--"])
        .current_dir(root)
        .output()
        .context("finding renamed touched file for LOC remediation")?;
    if !output.status.success() {
        bail!(
            "git diff for LOC remediation rename detection failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.first().is_some_and(|status| status.starts_with('R'))
            && fields
                .get(2)
                .is_some_and(|new_path| Path::new(new_path) == path)
        {
            return Ok(PathBuf::from(fields[1]));
        }
    }
    Ok(path.to_owned())
}

fn read_base_text(root: &Path, base_ref: &str, path: &Path) -> Result<Option<String>> {
    let spec = format!("{base_ref}:{}", path.to_string_lossy());
    let output = Command::new("git")
        .args(["show", &spec])
        .current_dir(root)
        .output()
        .context("reading baseline file for LOC remediation")?;
    if output.status.success() {
        return Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()));
    }
    if output.status.code() == Some(128) {
        return Ok(None);
    }
    bail!(
        "git show for LOC remediation baseline failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn nonempty_line_count(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

fn without_whitespace(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
