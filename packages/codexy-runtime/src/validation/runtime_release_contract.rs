mod checks;
pub(crate) mod core;

use std::path::Path;

pub(super) fn check(plugin_root: &Path, supported: &[String]) -> anyhow::Result<()> {
    checks::check(plugin_root, supported)
}
