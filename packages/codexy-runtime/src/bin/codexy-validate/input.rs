use std::path::PathBuf;

use anyhow::Result;

use super::Cli;

pub(super) fn merge_message(cli: &Cli) -> Result<String> {
    if let Some(message) = &cli.merge_message {
        return Ok(message.clone());
    }
    if let Some(path) = &cli.merge_message_file {
        return std::fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("reading {}: {error}", path.display()));
    }
    anyhow::bail!("--merge-message or --merge-message-file is required")
}

pub(super) fn child_lane_ownership_evidence(cli: &Cli) -> Result<String> {
    read_required_file(&cli.evidence_file, "--evidence-file")
}

pub(super) fn read_required_file(path: &Option<PathBuf>, flag: &str) -> Result<String> {
    let path = path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("{flag} is required"))?;
    std::fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("reading {}: {error}", path.display()))
}

pub(super) fn ensure_one_mode(cli: &Cli) -> Result<()> {
    let modes = [
        cli.check,
        cli.check_lsp,
        cli.check_rust_lsp_readiness,
        cli.check_merge_message,
        cli.check_merge_authorization,
        cli.check_pr_title,
        cli.check_issue_title,
        cli.check_issue_intake,
        cli.check_completion_handoff,
        cli.check_routing_measurement,
        cli.resolve_child_routing,
        cli.resolve_tdd_classification,
        cli.check_mcp,
        cli.check_hooks,
        cli.check_roles,
        cli.check_runtime_artifacts,
        cli.check_child_lane_ownership,
        cli.check_touched_loc,
        cli.print_covered_extensions,
        cli.print_density_inventory,
    ];
    if modes.into_iter().filter(|enabled| *enabled).count() != 1 {
        anyhow::bail!("exactly one validation mode is required");
    }
    Ok(())
}
