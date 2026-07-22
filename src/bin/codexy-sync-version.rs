use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(about = "Check or synchronize Codexy plugin version metadata.")]
struct Cli {
    #[arg(long, conflicts_with = "version")]
    check: bool,
    #[arg(long, requires = "check")]
    tag: Option<String>,
    #[arg(long)]
    version: Option<String>,
    #[arg(long, conflicts_with_all = ["check", "tag", "version"])]
    advance_bootstrap: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let message = if cli.check {
        codexy_runtime::version::check_versions_for_tag(cli.tag.as_deref())?
    } else if let Some(version) = cli.version {
        codexy_runtime::version::set_version(&version)?
    } else if cli.advance_bootstrap {
        codexy_runtime::version::advance_bootstrap()?
    } else {
        anyhow::bail!("one of --check or --version is required");
    };
    println!("{message}");
    Ok(())
}
