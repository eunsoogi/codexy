use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

use codexy_runtime::paths;
use codexy_runtime::validation::{self, Mode};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

pub(crate) fn validator(
    plugin_root: &Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    validator_in_process(plugin_root, mode)
}

pub(crate) fn validator_instruction_policy(
    plugin_root: &Path,
) -> Result<Output, Box<dyn std::error::Error>> {
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy");
    let mut changed = Vec::new();
    let used_fixture_manifest = match super::fixture_mutable_files(plugin_root) {
        Some(mutable_files) => {
            collect_declared_fixture_changes(plugin_root, &canonical, &mutable_files, &mut changed)?
        }
        None => false,
    };
    if !used_fixture_manifest {
        changed.clear();
        collect_changed_surfaces(plugin_root, &canonical, &mut changed)?;
    }
    if let Some(repo_root) = plugin_root.parent().and_then(Path::parent) {
        let current_agents = repo_root.join("AGENTS.md");
        let canonical_agents = Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md");
        if current_agents.is_file()
            && std::fs::read(&current_agents)? != std::fs::read(canonical_agents)?
        {
            changed.push(current_agents);
        }
    }
    if changed.is_empty() {
        return validator_in_process_mode(plugin_root, Mode::InstructionPolicy);
    }
    let mut errors = Vec::new();
    for path in changed {
        errors.extend(validation::instruction_policy_diagnostics(&path)?);
    }
    Ok(output_from_errors(plugin_root, errors))
}

fn collect_declared_fixture_changes(
    current: &Path,
    canonical: &Path,
    mutable_files: &[PathBuf],
    changed: &mut Vec<PathBuf>,
) -> std::io::Result<bool> {
    for relative in mutable_files {
        let current_path = current.join(relative);
        let canonical_path = canonical.join(relative);
        if !current_path.is_file() || !canonical_path.is_file() {
            return Ok(false);
        }
        if std::fs::read(&current_path)? != std::fs::read(&canonical_path)? {
            changed.push(current_path);
        }
    }
    Ok(true)
}

fn collect_changed_surfaces(
    current: &Path,
    canonical: &Path,
    changed: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let current_path = entry.path();
        let canonical_path = canonical.join(entry.file_name());
        if current_path.is_dir() {
            collect_changed_surfaces(&current_path, &canonical_path, changed)?;
        } else if std::fs::read(&current_path)?
            != std::fs::read(&canonical_path).unwrap_or_default()
        {
            changed.push(current_path);
        }
    }
    Ok(())
}

pub(crate) fn validator_routing(plugin_root: &Path) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process_mode(plugin_root, Mode::OrchestrationRouting)
}

pub(crate) fn validator_pr_labels(pr_state: &str) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process_mode(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        Mode::PrLabels {
            pr_state: pr_state.to_owned(),
        },
    )
}

pub(crate) fn validator_child_lane_ownership_file(
    evidence_path: &Path,
) -> Result<Output, Box<dyn std::error::Error>> {
    let evidence = std::fs::read_to_string(evidence_path)?;
    validator_in_process_mode(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        Mode::ChildLaneOwnership { evidence },
    )
}

pub(crate) fn validator_completion_handoff_files(
    handoff_path: &Path,
    pr_state_path: &Path,
) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process_mode(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        Mode::CompletionHandoff {
            handoff: std::fs::read_to_string(handoff_path)?,
            pr_state: std::fs::read_to_string(pr_state_path)?,
        },
    )
}

pub(crate) fn validator_in_process(
    plugin_root: &Path,
    mode: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let mode = match mode {
        "--check" => Mode::All,
        "--check-mcp" => Mode::Mcp,
        "--check-roles" => Mode::Roles,
        _ => return Err(format!("unsupported in-process validation mode: {mode}").into()),
    };
    validator_in_process_mode(plugin_root, mode)
}

fn validator_in_process_mode(
    plugin_root: &Path,
    mode: Mode,
) -> Result<Output, Box<dyn std::error::Error>> {
    super::profile_metrics::record("validator_in_process");
    let errors = validation::errors(plugin_root, mode);
    Ok(output_from_errors(plugin_root, errors))
}

fn output_from_errors(plugin_root: &Path, errors: Vec<String>) -> Output {
    let stderr = errors
        .iter()
        .map(|error| format!("error: {error}"))
        .chain((!errors.is_empty()).then(|| {
            format!(
                "Error: plugin validation failed with {} error(s)",
                errors.len()
            )
        }))
        .collect::<Vec<_>>()
        .join("\n");
    Output {
        status: exit_status(errors.is_empty()),
        stdout: errors
            .is_empty()
            .then(|| {
                format!(
                    "plugin config validation ok: {}\n",
                    paths::display_relative(plugin_root)
                )
            })
            .unwrap_or_default()
            .into_bytes(),
        stderr: (!stderr.is_empty())
            .then(|| format!("{stderr}\n"))
            .unwrap_or_default()
            .into_bytes(),
    }
}

fn exit_status(success: bool) -> ExitStatus {
    #[cfg(unix)]
    return ExitStatus::from_raw(if success { 0 } else { 1 << 8 });
    #[cfg(windows)]
    ExitStatus::from_raw(u32::from(!success))
}

pub(crate) fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
