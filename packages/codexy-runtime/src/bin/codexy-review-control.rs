use std::{fs, path::PathBuf};

use anyhow::Result;
use clap::Parser;

use codexy_runtime::{paths, validation};

#[derive(Debug, Parser)]
#[command(about = "Resolve and validate Codexy bounded-review contracts.")]
struct Cli {
    #[arg(long)]
    plugin_root: Option<PathBuf>,
    #[arg(long)]
    repository_root: Option<PathBuf>,
    #[arg(long)]
    ledger: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["check_packet", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility"])]
    resolve_profile: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility"])]
    check_packet: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility"])]
    check_economics: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility"])]
    capture_economics: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility"])]
    build_pr_state: bool,
    #[arg(long, visible_alias = "capture-review-control", conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "build_pr_state", "import_pre_pr_history", "check_next_review_eligibility"])]
    produce_review_control: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "check_next_review_eligibility"])]
    import_pre_pr_history: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history"])]
    check_next_review_eligibility: bool,
    #[arg(long)]
    input: Option<PathBuf>,
    #[arg(long, visible_alias = "current-pr-state-file")]
    base_pr_state_file: Option<PathBuf>,
    #[arg(long)]
    review_control_state_file: Option<PathBuf>,
    #[arg(long)]
    previous_pr_state_file: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    observer_command: Option<PathBuf>,
    #[arg(long)]
    trusted_receipt: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.plugin_root.unwrap_or_else(paths::plugin_root);
    if cli.check_next_review_eligibility {
        let repository_root = cli
            .repository_root
            .unwrap_or_else(|| paths::repository_root().to_path_buf());
        let current = fs::read_to_string(
            cli.base_pr_state_file
                .ok_or_else(|| anyhow::anyhow!("--current-pr-state-file is required"))?,
        )?;
        let previous = fs::read_to_string(
            cli.previous_pr_state_file
                .ok_or_else(|| anyhow::anyhow!("--previous-pr-state-file is required"))?,
        )?;
        let request = fs::read_to_string(
            cli.input
                .ok_or_else(|| anyhow::anyhow!("--input is required"))?,
        )?;
        let output = cli
            .output
            .ok_or_else(|| anyhow::anyhow!("--output is required"))?;
        let receipt = validation::check_next_review_eligibility(
            &root,
            &repository_root,
            &current,
            &previous,
            &request,
        )?;
        fs::write(output, serde_json::to_vec_pretty(&receipt)?)?;
    } else if cli.import_pre_pr_history {
        let current = fs::read_to_string(cli.base_pr_state_file.ok_or_else(|| {
            anyhow::anyhow!("pre-PR history import requires --current-pr-state-file")
        })?)?;
        let envelope = fs::read_to_string(
            cli.input
                .ok_or_else(|| anyhow::anyhow!("pre-PR history import requires --input"))?,
        )?;
        let output = cli
            .output
            .ok_or_else(|| anyhow::anyhow!("pre-PR history import requires --output"))?;
        let state = validation::import_pre_pr_review_history(
            &root,
            &cli.repository_root
                .unwrap_or_else(|| paths::repository_root().to_path_buf()),
            &current,
            &envelope,
        )?;
        fs::write(output, serde_json::to_vec_pretty(&state)?)?;
    } else if cli.produce_review_control {
        let input = fs::read_to_string(
            cli.input
                .ok_or_else(|| anyhow::anyhow!("review-control producer requires --input"))?,
        )?;
        let output = cli
            .output
            .ok_or_else(|| anyhow::anyhow!("review-control producer requires --output"))?;
        let produced = validation::produce_review_control(
            &root,
            &cli.repository_root
                .unwrap_or_else(|| paths::repository_root().to_path_buf()),
            &input,
        )?;
        fs::write(
            output,
            serde_json::to_vec_pretty(&produced["control_state"])?,
        )?;
    } else if cli.capture_economics {
        let request = serde_json::json!({
            "schema":"codexy.review-economics-capture-request.v1",
            "observer_command":cli.observer_command.ok_or_else(|| anyhow::anyhow!("--observer-command is required"))?,
            "trusted_receipt":cli.trusted_receipt.ok_or_else(|| anyhow::anyhow!("--trusted-receipt is required"))?,
            "output":cli.output.ok_or_else(|| anyhow::anyhow!("--output is required"))?
        });
        validation::check_review_economics(
            &root,
            &cli.repository_root
                .unwrap_or_else(|| paths::repository_root().to_path_buf()),
            &serde_json::to_string(&request)?,
        )?;
    } else if cli.build_pr_state {
        let repository_root = cli
            .repository_root
            .unwrap_or_else(|| paths::repository_root().to_path_buf());
        let base = fs::read_to_string(
            cli.base_pr_state_file
                .ok_or_else(|| anyhow::anyhow!("--base-pr-state-file is required"))?,
        )?;
        let control = fs::read_to_string(
            cli.review_control_state_file
                .ok_or_else(|| anyhow::anyhow!("--review-control-state-file is required"))?,
        )?;
        let previous = fs::read_to_string(
            cli.previous_pr_state_file
                .ok_or_else(|| anyhow::anyhow!("--previous-pr-state-file is required"))?,
        )?;
        let output = cli
            .output
            .ok_or_else(|| anyhow::anyhow!("--output is required"))?;
        fs::write(
            output,
            serde_json::to_vec(&validation::build_review_pr_state(
                &root,
                &repository_root,
                &base,
                &control,
                &previous,
            )?)?,
        )?;
    } else if cli.resolve_profile {
        let input = fs::read_to_string(
            cli.input
                .ok_or_else(|| anyhow::anyhow!("--input is required"))?,
        )?;
        println!(
            "{}",
            serde_json::to_string(&validation::resolve_review_profile(&root, &input)?)?
        );
    } else if cli.check_packet {
        let input = fs::read_to_string(
            cli.input
                .ok_or_else(|| anyhow::anyhow!("--input is required"))?,
        )?;
        validation::check_review_packet(
            &root,
            &cli.repository_root
                .unwrap_or_else(|| paths::repository_root().to_path_buf()),
            &cli.ledger
                .ok_or_else(|| anyhow::anyhow!("--ledger is required with --check-packet"))?,
            &input,
        )?;
        println!("review packet validation ok");
    } else if cli.check_economics {
        let input = fs::read_to_string(
            cli.input
                .ok_or_else(|| anyhow::anyhow!("--input is required"))?,
        )?;
        validation::check_review_economics(
            &root,
            &cli.repository_root
                .unwrap_or_else(|| paths::repository_root().to_path_buf()),
            &input,
        )?;
        println!("review economics validation ok");
    } else {
        anyhow::bail!("exactly one review-control mode is required");
    }
    Ok(())
}
