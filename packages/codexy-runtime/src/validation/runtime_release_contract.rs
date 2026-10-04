mod checks;
pub(crate) mod core;

use std::path::Path;

pub(super) fn check(plugin_root: &Path, supported: &[String]) -> anyhow::Result<()> {
    // Keep the public validator entrypoint stable while the receipt checks evolve in focused submodules.
    checks::check(plugin_root, supported)
}
