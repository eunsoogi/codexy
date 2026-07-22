use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

mod changes;
mod reconciliation;

use super::touched_loc_remediation;

const LOC_LIMIT: usize = 250;
const EXCEPTIONS_PATH: &str = ".codexy-loc-exceptions";

pub(super) fn check(base_ref: &str) -> Vec<String> {
    match check_inner(base_ref) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}

fn check_inner(base_ref: &str) -> Result<()> {
    let root = git_top_level()?;
    let errors = diagnostics_at(&root, base_ref)?;
    if errors.is_empty() {
        return Ok(());
    }
    for error in &errors {
        eprintln!("error: {error}");
    }
    bail!(
        "touched LOC validation failed with {} error(s)",
        errors.len()
    )
}

pub(super) fn diagnostics_at(root: &Path, base_ref: &str) -> Result<Vec<String>> {
    let mut errors = Vec::new();
    if root.join(EXCEPTIONS_PATH).exists() {
        errors.push(format!(
            "{EXCEPTIONS_PATH} is not supported; every governed file must stay at or below {LOC_LIMIT} lines"
        ));
    }
    for file in changes::scoped(&root, base_ref)? {
        if !is_governed_path(&file.path) {
            continue;
        }
        let line_count = count_lines(&root.join(&file.path))?;
        if let Some(error) = touched_loc_remediation::formatting_only_error(
            &root,
            base_ref,
            &file.baseline,
            &file.path,
            line_count,
            LOC_LIMIT,
        )? {
            errors.push(error);
            continue;
        }
    }
    for path in governed_files(&root)? {
        let line_count = count_lines(&root.join(&path))?;
        if line_count > LOC_LIMIT {
            errors.push(format!(
                "{} has {line_count} lines; governed human-authored files must stay at or below {LOC_LIMIT} lines",
                path.display()
            ));
        }
    }
    Ok(errors)
}

fn git_top_level() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("resolving git top-level for touched LOC validation")?;
    if !output.status.success() {
        bail!(
            "git rev-parse for touched LOC validation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

pub(super) fn changed_files(root: &Path, base_ref: &str) -> Result<Vec<PathBuf>> {
    Ok(changes::scoped(root, base_ref)?
        .into_iter()
        .map(|file| file.path)
        .collect())
}

fn count_lines(path: &Path) -> Result<usize> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading touched file {}", path.display()))?;
    Ok(text.lines().count())
}

fn governed_files(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .current_dir(root)
        .output()
        .context("listing governed files for LOC validation")?;
    if !output.status.success() {
        bail!(
            "git ls-files for LOC validation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let mut files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .filter(|path| is_governed_path(path))
        .filter(|path| root.join(path).is_file())
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    Ok(files)
}

fn is_governed_path(path: &Path) -> bool {
    let path_text = path.to_string_lossy();
    if path_text.starts_with("target/") || path_text.starts_with(".git/") {
        return false;
    }
    if path.file_name().and_then(|name| name.to_str()) == Some("AGENTS.md") {
        return true;
    }
    if path_text.starts_with("plugins/codexy/skills/")
        && path.extension().and_then(|extension| extension.to_str()) == Some("md")
    {
        return true;
    }
    if path_text.starts_with(".github/workflows/")
        && matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yml" | "yaml")
        )
    {
        return true;
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("rs" | "sh" | "py" | "js" | "ts" | "tsx" | "jsx")
    ) || path_text.starts_with("plugins/codexy/mcp/")
        || path_text.starts_with("plugins/codexy/hooks/")
        || path_text.starts_with("scripts/")
}
