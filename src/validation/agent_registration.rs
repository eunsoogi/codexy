#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use crate::paths::display_relative;

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let script = plugin_root.join("skills/codex-orchestration/scripts/register-codexy-agents");
    let mut errors = Vec::new();
    if !script.is_file() {
        errors.push(format!(
            "{} must exist to register plugin-packaged agents through Codex [agents.<name>] config_file",
            display_relative(&script)
        ));
        return errors;
    }
    #[cfg(unix)]
    if script
        .metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 == 0)
        .unwrap_or(true)
    {
        errors.push(format!("{} must be executable", display_relative(&script)));
    }
    errors
}
