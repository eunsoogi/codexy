use std::{fs, path::PathBuf};

use anyhow::Result;
use clap::Parser;

use codexy_runtime::{paths, validation};

#[derive(Debug, Parser)]
#[command(about = "Resolve and validate Codexy bounded-review contracts.")]
struct Cli {
    #[arg(long)]
    plugin_root: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["check_packet", "check_economics"])]
    resolve_profile: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_economics"])]
    check_packet: bool,
    #[arg(long, conflicts_with_all = ["resolve_profile", "check_packet"])]
    check_economics: bool,
    #[arg(long)]
    input: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let input = fs::read_to_string(
        cli.input
            .ok_or_else(|| anyhow::anyhow!("--input is required"))?,
    )?;
    let root = cli.plugin_root.unwrap_or_else(paths::plugin_root);
    if cli.resolve_profile {
        println!(
            "{}",
            serde_json::to_string(&validation::resolve_review_profile(&root, &input)?)?
        );
    } else if cli.check_packet {
        validation::check_review_packet(&root, &input)?;
        println!("review packet validation ok");
    } else if cli.check_economics {
        validation::check_review_economics(&root, &input)?;
        println!("review economics validation ok");
    } else {
        anyhow::bail!("exactly one review-control mode is required");
    }
    Ok(())
}
