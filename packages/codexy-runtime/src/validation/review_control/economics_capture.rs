use std::path::Path;

use anyhow::{Result, bail};

use super::economics::UNAVAILABLE_REASON;

pub(super) fn capture(
    plugin_root: &Path,
    repository_root: &Path,
    observer_command: &Path,
    trusted_receipt: &Path,
    output: &Path,
) -> Result<()> {
    let _ = (
        plugin_root,
        repository_root,
        observer_command,
        trusted_receipt,
        output,
    );
    bail!(UNAVAILABLE_REASON)
}
