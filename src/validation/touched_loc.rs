use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

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
    let exceptions = load_exceptions(&root)?;
    let mut errors = Vec::new();
    for path in changed_files(&root, base_ref)? {
        if !is_implementation_path(&path) {
            continue;
        }
        let line_count = count_lines(&root.join(&path))?;
        if let Some(error) = touched_loc_remediation::formatting_only_error(
            &root, base_ref, &path, line_count, LOC_LIMIT,
        )? {
            errors.push(error);
            continue;
        }
        if line_count <= LOC_LIMIT || exceptions.contains_key(&path) {
            continue;
        }
        errors.push(format!(
            "{} has {line_count} lines; touched implementation/test harness files must stay at or below {LOC_LIMIT} LOC",
            path.display()
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        for error in &errors {
            eprintln!("error: {error}");
        }
        bail!(
            "touched LOC validation failed with {} error(s)",
            errors.len()
        )
    }
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
    let mut files = run_git_diff(root, &format!("{base_ref}...HEAD"))?;
    files.extend(run_git_diff(root, "--cached")?);
    files.extend(run_git_diff(root, "")?);
    files.extend(untracked_files(root)?);
    files.sort();
    files.dedup();
    Ok(files)
}

fn run_git_diff(root: &Path, range: &str) -> Result<Vec<PathBuf>> {
    let mut command = Command::new("git");
    command.args(["diff", "--name-only", "--diff-filter=ACMRT"]);
    if !range.is_empty() {
        command.arg(range);
    }
    let output = command
        .current_dir(root)
        .output()
        .context("running git diff for touched LOC validation")?;
    if !output.status.success() {
        bail!(
            "git diff for touched LOC validation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect())
}

fn untracked_files(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(root)
        .output()
        .context("running git ls-files for touched LOC validation")?;
    if !output.status.success() {
        bail!(
            "git ls-files for touched LOC validation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect())
}

fn count_lines(path: &Path) -> Result<usize> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading touched file {}", path.display()))?;
    Ok(text.lines().count())
}

fn is_implementation_path(path: &Path) -> bool {
    let path_text = path.to_string_lossy();
    if path_text.starts_with("target/") || path_text.starts_with(".git/") {
        return false;
    }
    if path_text.starts_with("plugins/codexy/skills/")
        && path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
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

fn load_exceptions(root: &Path) -> Result<BTreeMap<PathBuf, String>> {
    let path = root.join(EXCEPTIONS_PATH);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    ensure_tracked_exception_file(root)?;
    ensure_clean_exception_file(root)?;
    let mut exceptions = BTreeMap::new();
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", EXCEPTIONS_PATH))?;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((file, reason)) = trimmed.split_once(char::is_whitespace) else {
            bail!(
                "{}:{} must include a path and rationale",
                EXCEPTIONS_PATH,
                index + 1
            );
        };
        let reason = reason.trim();
        if reason.len() < 12 {
            bail!("{}:{} rationale is too short", EXCEPTIONS_PATH, index + 1);
        }
        exceptions.insert(PathBuf::from(file), reason.to_owned());
    }
    Ok(exceptions)
}

fn ensure_tracked_exception_file(root: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["ls-files", "--error-unmatch", EXCEPTIONS_PATH])
        .current_dir(root)
        .output()
        .context("checking tracked LOC exception file")?;
    if output.status.success() {
        return Ok(());
    }
    bail!("{EXCEPTIONS_PATH} must be tracked before it can exempt oversized touched files")
}

fn ensure_clean_exception_file(root: &Path) -> Result<()> {
    ensure_no_exception_diff(root, false)?;
    ensure_no_exception_diff(root, true)?;
    Ok(())
}

fn ensure_no_exception_diff(root: &Path, cached: bool) -> Result<()> {
    let mut command = Command::new("git");
    command.arg("diff");
    if cached {
        command.arg("--cached");
    }
    command.args(["--quiet", "--", EXCEPTIONS_PATH]);
    let output = command
        .current_dir(root)
        .output()
        .context("checking clean LOC exception file")?;
    if output.status.success() {
        return Ok(());
    }
    if output.status.code() == Some(1) {
        bail!(
            "{EXCEPTIONS_PATH} has uncommitted changes; commit or discard them before it can exempt oversized touched files"
        );
    }
    bail!(
        "git diff for LOC exception file failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}
