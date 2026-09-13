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
    #[arg(long, conflicts_with_all = ["check_packet", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility", "recover_native_history"])]
    resolve_profile: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility", "recover_native_history"])]
    check_packet: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility", "recover_native_history"])]
    check_economics: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility", "recover_native_history"])]
    capture_economics: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility", "recover_native_history"])]
    build_pr_state: bool,
    #[arg(long, visible_alias = "capture-review-control", conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "build_pr_state", "import_pre_pr_history", "check_next_review_eligibility", "recover_native_history"])]
    produce_review_control: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "check_next_review_eligibility", "recover_native_history"])]
    import_pre_pr_history: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "recover_native_history"])]
    check_next_review_eligibility: bool,
    #[arg(long = "recover-native-review-history", conflicts_with_all = ["resolve_profile", "check_packet", "check_economics", "capture_economics", "build_pr_state", "produce_review_control", "import_pre_pr_history", "check_next_review_eligibility"])]
    recover_native_history: bool,
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
    if cli.recover_native_history {
        anyhow::bail!(
            "legacy review-control processing is no longer supported: native review-history recovery is retired"
        );
    }
    if cli.check_next_review_eligibility {
        anyhow::bail!(
            "legacy review-control processing is no longer supported: next-review eligibility is retired"
        );
    }
    if cli.import_pre_pr_history {
        anyhow::bail!(
            "legacy review-control processing is no longer supported: pre-PR history import is retired"
        );
    }
    if cli.capture_economics {
        anyhow::bail!(
            "legacy review-control processing is no longer supported: review economics capture is retired"
        );
    }
    if cli.check_packet {
        anyhow::bail!(
            "legacy review-control processing is no longer supported: review packet validation is retired"
        );
    }
    if cli.check_economics {
        anyhow::bail!(
            "legacy review-control processing is no longer supported: review economics validation is retired"
        );
    }

    let root = cli.plugin_root.unwrap_or_else(paths::plugin_root);
    if cli.produce_review_control {
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
        let previous = match cli.previous_pr_state_file {
            Some(path) => fs::read_to_string(path)?,
            None => "{}".to_owned(),
        };
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
    } else {
        anyhow::bail!("exactly one review-control mode is required");
    }
    Ok(())
}
