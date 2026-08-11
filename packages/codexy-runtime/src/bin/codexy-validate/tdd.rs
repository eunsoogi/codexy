use std::path::Path;

use anyhow::Result;

use codexy_runtime::validation;

use super::{Cli, read_required_file};

pub(super) fn emit_resolution(cli: &Cli, plugin_root: &Path) -> Result<bool> {
    if !cli.resolve_tdd_classification {
        return Ok(false);
    }
    let request = read_required_file(
        &cli.tdd_classification_request_file,
        "--tdd-classification-request-file",
    )?;
    println!(
        "{}",
        serde_json::to_string(&validation::resolve_tdd_classification(
            plugin_root,
            &request
        )?)?
    );
    Ok(true)
}
