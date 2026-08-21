use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(about = "Check or synchronize Codexy plugin version metadata.")]
struct Cli {
    #[arg(long, conflicts_with_all = [
        "version",
        "admit_version",
        "admit_candidate",
        "prepare_candidate",
        "check_candidate"
    ])]
    check: bool,
    #[arg(long, requires = "check")]
    tag: Option<String>,
    #[arg(long, conflicts_with_all = [
        "admit_version",
        "admit_candidate",
        "prepare_candidate",
        "check_candidate"
    ])]
    version: Option<String>,
    #[arg(long, conflicts_with_all = ["admit_candidate", "prepare_candidate", "check_candidate"])]
    admit_version: Option<String>,
    #[arg(long, conflicts_with_all = ["prepare_candidate", "check_candidate"])]
    admit_candidate: Option<String>,
    #[arg(long, conflicts_with = "check_candidate")]
    prepare_candidate: Option<String>,
    #[arg(long)]
    check_candidate: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let message = if let Some(version) = cli.admit_candidate {
        format!(
            "candidate version admission ok: {:?}",
            codexy_runtime::version::admit_candidate(&version)?
        )
    } else if let Some(version) = cli.prepare_candidate {
        codexy_runtime::version::prepare_candidate(&version)?
    } else if cli.check_candidate {
        codexy_runtime::version::check_candidate()?
    } else if let Some(version) = cli.admit_version {
        format!(
            "version advance admission ok: {:?}",
            codexy_runtime::version::admit(&version)?
        )
    } else if cli.check {
        codexy_runtime::version::check_versions_for_tag(cli.tag.as_deref())?
    } else if let Some(version) = cli.version {
        codexy_runtime::version::set_version(&version)?
    } else {
        anyhow::bail!(
            "one of --check, --check-candidate, --admit-version, --admit-candidate, --prepare-candidate, or --version is required"
        );
    };
    println!("{message}");
    Ok(())
}
