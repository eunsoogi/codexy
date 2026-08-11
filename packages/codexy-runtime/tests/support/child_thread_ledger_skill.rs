use std::process::{ExitStatus, Output};

use codexy_runtime::paths;
use codexy_runtime::validation::{self, Mode};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

pub(crate) fn validator(
    plugin_root: &std::path::Path,
    mode: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process(plugin_root, mode)
}

pub(crate) fn validator_pr_labels(pr_state: &str) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process_mode(
        &codexy_runtime::paths::repository_root().join("plugins/codexy"),
        Mode::PrLabels {
            pr_state: pr_state.to_owned(),
        },
    )
}

pub(crate) fn validator_child_lane_ownership_file(
    evidence_path: &std::path::Path,
) -> Result<Output, Box<dyn std::error::Error>> {
    validator_child_lane_ownership(&std::fs::read_to_string(evidence_path)?)
}

pub(crate) fn validator_child_lane_ownership(
    evidence: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process_mode(
        &codexy_runtime::paths::repository_root().join("plugins/codexy"),
        Mode::ChildLaneOwnership {
            evidence: evidence.to_owned(),
        },
    )
}

pub(crate) fn validator_completion_handoff_files(
    handoff_path: &std::path::Path,
    pr_state_path: &std::path::Path,
) -> Result<Output, Box<dyn std::error::Error>> {
    validator_completion_handoff(
        &std::fs::read_to_string(handoff_path)?,
        &std::fs::read_to_string(pr_state_path)?,
    )
}

pub(crate) fn validator_completion_handoff(
    handoff: &str,
    pr_state: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process_mode(
        &codexy_runtime::paths::repository_root().join("plugins/codexy"),
        Mode::CompletionHandoff {
            handoff: handoff.to_owned(),
            pr_state: pr_state.to_owned(),
        },
    )
}

pub(crate) fn validator_in_process(
    plugin_root: &std::path::Path,
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
    plugin_root: &std::path::Path,
    mode: Mode,
) -> Result<Output, Box<dyn std::error::Error>> {
    super::profile_metrics::record("validator_in_process");
    Ok(output_from_errors(
        plugin_root,
        validation::errors(plugin_root, mode),
    ))
}

fn output_from_errors(plugin_root: &std::path::Path, errors: Vec<String>) -> Output {
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

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
