use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(about = "Check or synchronize Codexy plugin version metadata.")]
struct Cli {
    #[arg(long, conflicts_with_all = ["version", "admit_version"])]
    check: bool,
    #[arg(long, requires = "check")]
    tag: Option<String>,
    #[arg(long, conflicts_with = "admit_version")]
    version: Option<String>,
    #[arg(long)]
    admit_version: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let message = if let Some(version) = cli.admit_version {
        format!(
            "version advance admission ok: {:?}",
            codexy_runtime::version::admit(&version)?
        )
    } else if cli.check {
        codexy_runtime::version::check_versions_for_tag(cli.tag.as_deref())?
    } else if let Some(version) = cli.version {
        codexy_runtime::version::set_version(&version)?
    } else {
        anyhow::bail!("one of --check, --admit-version, or --version is required");
    };
    println!("{message}");
    Ok(())
}
