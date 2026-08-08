use std::path::Path;

use anyhow::Result;

/// Returns orchestration-routing diagnostics for one explicit skill file.
///
/// This pure semantic adapter lets permutation tests avoid copying a complete
/// plugin fixture and spawning the validator process. CLI tests retain that
/// process-level contract separately.
///
/// # Errors
///
/// Returns an error when the skill file cannot be read.
pub fn diagnostics(path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)?;
    Ok(super::orchestration_routing::check_skill(path, &text))
}
