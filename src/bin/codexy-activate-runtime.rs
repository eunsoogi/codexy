use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(about = "Apply a verified runtime activation without publishing it.")]
struct Cli {
    #[arg(long)]
    repo_root: PathBuf,
    #[arg(long)]
    bootstrap_version: String,
    #[arg(long)]
    candidate_receipt: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let count = codexy_runtime::version::activation::activate(
        &cli.repo_root,
        &cli.bootstrap_version,
        &cli.candidate_receipt,
    )?;
    println!(
        "runtime activation updated {count} allowlisted files; publication and pull request creation remain external"
    );
    Ok(())
}
