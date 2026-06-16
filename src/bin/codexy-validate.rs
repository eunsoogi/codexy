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
    #[arg(long, conflicts_with_all = ["check_lsp", "check_mcp", "check_roles", "check_runtime_artifacts", "print_covered_extensions"])]
    check: bool,
    #[arg(long, conflicts_with_all = ["check", "check_mcp", "check_roles", "check_runtime_artifacts", "print_covered_extensions"])]
    check_lsp: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_roles", "check_runtime_artifacts", "print_covered_extensions"])]
    check_mcp: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_mcp", "check_runtime_artifacts", "print_covered_extensions"])]
    check_roles: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_mcp", "check_roles", "print_covered_extensions"])]
    check_runtime_artifacts: bool,
    #[arg(long, conflicts_with_all = ["check", "check_lsp", "check_mcp", "check_roles", "check_runtime_artifacts"])]
    print_covered_extensions: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let plugin_root = cli.plugin_root.unwrap_or_else(paths::plugin_root);
    if cli.print_covered_extensions {
        for extension in validation::covered_extensions(&plugin_root)? {
            println!("{extension}");
        }
        return Ok(());
    }
    let mode = if cli.check_lsp {
        validation::Mode::Lsp
    } else if cli.check_mcp {
        validation::Mode::Mcp
    } else if cli.check_roles {
        validation::Mode::Roles
    } else if cli.check_runtime_artifacts {
        validation::Mode::RuntimeArtifacts
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
