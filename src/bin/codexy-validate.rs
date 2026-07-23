use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use codexy_runtime::{paths, validation};

#[derive(Debug, Parser)]
#[command(about = "Validate Codexy plugin configuration surfaces.")]
#[allow(clippy::struct_excessive_bools)]
struct Cli {
    #[arg(long)]
    plugin_root: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check: bool,
    #[arg(long, conflicts_with_all = ["check", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_lsp: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_rust_lsp_readiness: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_merge_message: bool,
    #[arg(long, requires = "check_merge_message")]
    expected_issue: Option<u64>,
    #[arg(long, requires = "check_merge_message")]
    expected_pr: Option<u64>,
    #[arg(
        long,
        requires = "check_merge_message",
        conflicts_with = "merge_message_file"
    )]
    merge_message: Option<String>,
    #[arg(
        long,
        requires = "check_merge_message",
        conflicts_with = "merge_message"
    )]
    merge_message_file: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_pr_title: bool,
    #[arg(long, requires = "check_pr_title")]
    pr_title: Option<String>,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_issue_title: bool,
    #[arg(long, requires = "check_issue_title")]
    issue_title: Option<String>,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_issue_intake: bool,
    #[arg(long, requires = "check_issue_intake")]
    issue_intake_file: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_completion_handoff: bool,
    #[arg(long, requires = "check_completion_handoff")]
    handoff_file: Option<PathBuf>,
    #[arg(long, requires = "check_completion_handoff")]
    pr_state_file: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_issue_intake", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_review_response_cluster: bool,
    #[arg(long, requires = "check_review_response_cluster")]
    review_response_cluster_file: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_mcp: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_hooks: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_roles: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_child_lane_ownership", "check_touched_loc", "print_covered_extensions"])]
    check_runtime_artifacts: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_touched_loc", "print_covered_extensions"])]
    check_child_lane_ownership: bool,
    #[arg(long, requires = "check_child_lane_ownership")]
    evidence_file: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "print_covered_extensions"])]
    check_touched_loc: bool,
    #[arg(long, requires = "check_touched_loc", default_value = "origin/main")]
    base_ref: String,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_rust_lsp_readiness", "check_merge_message", "check_pr_title", "check_issue_title", "check_completion_handoff", "check_mcp", "check_hooks", "check_roles", "check_runtime_artifacts", "check_child_lane_ownership", "check_touched_loc"])]
    print_covered_extensions: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let plugin_root = cli.plugin_root.clone().unwrap_or_else(paths::plugin_root);
    if cli.print_covered_extensions {
        for extension in validation::covered_extensions(&plugin_root)? {
            println!("{extension}");
        }
        return Ok(());
    }
    let mode = if cli.check_lsp {
        validation::Mode::Lsp
    } else if cli.check_rust_lsp_readiness {
        validation::Mode::RustLspReadiness
    } else if cli.check_merge_message {
        if cli.expected_issue.is_none() && cli.expected_pr.is_none() {
            anyhow::bail!("--expected-issue or --expected-pr is required");
        }
        validation::Mode::MergeMessage {
            expected_issue: cli.expected_issue,
            expected_pr: cli.expected_pr,
            message: merge_message(&cli)?,
        }
    } else if cli.check_pr_title {
        validation::Mode::PrTitle {
            title: cli
                .pr_title
                .clone()
                .ok_or_else(|| anyhow::anyhow!("--pr-title is required"))?,
        }
    } else if cli.check_issue_title {
        validation::Mode::IssueTitle {
            title: cli
                .issue_title
                .clone()
                .ok_or_else(|| anyhow::anyhow!("--issue-title is required"))?,
        }
    } else if cli.check_issue_intake {
        validation::Mode::IssueIntake {
            receipt: read_required_file(&cli.issue_intake_file, "--issue-intake-file")?,
        }
    } else if cli.check_completion_handoff {
        validation::Mode::CompletionHandoff {
            handoff: read_required_file(&cli.handoff_file, "--handoff-file")?,
            pr_state: read_required_file(&cli.pr_state_file, "--pr-state-file")?,
        }
    } else if cli.check_review_response_cluster {
        validation::Mode::ReviewResponseCluster(read_required_file(
            &cli.review_response_cluster_file,
            "--review-response-cluster-file",
        )?)
    } else if cli.check_mcp {
        validation::Mode::Mcp
    } else if cli.check_hooks {
        validation::Mode::Hooks
    } else if cli.check_roles {
        validation::Mode::Roles
    } else if cli.check_runtime_artifacts {
        validation::Mode::RuntimeArtifacts
    } else if cli.check_child_lane_ownership {
        validation::Mode::ChildLaneOwnership {
            evidence: child_lane_ownership_evidence(&cli)?,
        }
    } else if cli.check_touched_loc {
        validation::Mode::TouchedLoc {
            base_ref: cli.base_ref,
        }
    } else if cli.check {
        validation::Mode::All
    } else {
        anyhow::bail!("one validation mode is required");
    };
    validation::run(&plugin_root, mode)?;
    println!(
        "plugin config validation ok: {}",
        paths::display_relative(&plugin_root)
    );
    Ok(())
}

fn merge_message(cli: &Cli) -> Result<String> {
    if let Some(message) = &cli.merge_message {
        return Ok(message.clone());
    }
    if let Some(path) = &cli.merge_message_file {
        return std::fs::read_to_string(path)
            .map_err(|error| anyhow::anyhow!("reading {}: {error}", path.display()));
    }
    anyhow::bail!("--merge-message or --merge-message-file is required")
}

fn child_lane_ownership_evidence(cli: &Cli) -> Result<String> {
    read_required_file(&cli.evidence_file, "--evidence-file")
}

fn read_required_file(path: &Option<PathBuf>, flag: &str) -> Result<String> {
    let path = path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("{flag} is required"))?;
    std::fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("reading {}: {error}", path.display()))
}
