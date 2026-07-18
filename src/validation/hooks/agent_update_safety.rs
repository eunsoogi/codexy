use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

const EXPECTED_HOOK: &str = include_str!("../../../plugins/codexy/hooks/codexy-routing-context.sh");
const EXPECTED_CHECKER: &str = include_str!("../../../plugins/codexy/check-codexy-agents");
const ROOT_ASSIGNMENT: &str = "plugin_root=$(CDPATH= cd -- \"$script_dir/..\" && pwd -P)";
const READ_ONLY_CALL: &str = "if registration_check=$(\"$plugin_root/check-codexy-agents\"); then";

pub(super) fn check(path: &Path, event: &str, script_path: &Path, text: &str) -> Result<()> {
    if script_path.file_name().and_then(|name| name.to_str()) != Some("codexy-routing-context.sh") {
        return Ok(());
    }
    if text != EXPECTED_HOOK {
        bail!(
            "{} {event} routing hook must match the compiled read-only package before execution: {}",
            display_relative(path),
            display_relative(script_path)
        );
    }
    let checker_lines: Vec<_> = text
        .lines()
        .filter(|line| line.contains("check-codexy-agents"))
        .map(str::trim)
        .collect();
    let root_assignments = text
        .lines()
        .filter(|line| line.trim() == ROOT_ASSIGNMENT)
        .count();
    let unsafe_computation = text.lines().map(str::trim).any(|line| {
        (line.contains("$(") && line != ROOT_ASSIGNMENT && line != READ_ONLY_CALL)
            || line.contains('`')
            || ((line.starts_with("\"$") || line.starts_with("\"${")) && line != READ_ONLY_CALL)
    });
    if root_assignments != 1
        || checker_lines.as_slice() != [READ_ONLY_CALL]
        || text.contains("bootstrap-codexy-agents")
        || unsafe_computation
    {
        bail!(
            "{} {event} routing hook must contain only the strict read-only agent update check: {}",
            display_relative(path),
            display_relative(script_path)
        );
    }
    let plugin_root = script_path
        .parent()
        .and_then(Path::parent)
        .context("routing hook must be inside the plugin hooks directory")?;
    check_exact(
        path,
        event,
        &plugin_root.join("check-codexy-agents"),
        EXPECTED_CHECKER,
        "checker",
    )
}

fn check_exact(
    path: &Path,
    event: &str,
    entrypoint: &Path,
    expected: &str,
    label: &str,
) -> Result<()> {
    if entrypoint
        .symlink_metadata()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(true)
        || !entrypoint.is_file()
    {
        bail!(
            "{} {event} agent update {label} must be a regular packaged file: {}",
            display_relative(path),
            display_relative(entrypoint)
        );
    }
    let actual = std::fs::read_to_string(entrypoint)
        .with_context(|| format!("reading {}", display_relative(entrypoint)))?;
    if actual != expected {
        bail!(
            "{} {event} agent update {label} must match the compiled read-only package: {}",
            display_relative(path),
            display_relative(entrypoint)
        );
    }
    Ok(())
}
