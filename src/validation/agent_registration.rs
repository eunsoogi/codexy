#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use crate::paths::display_relative;

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let script = plugin_root.join("skills/codex-orchestration/scripts/register-codexy-agents");
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");
    let mut errors = Vec::new();
    if !script.is_file() {
        errors.push(format!(
            "{} must exist to register plugin-packaged agents through Codex [agents.<name>] config_file",
            display_relative(&script)
        ));
        return errors;
    }
    let scripts = script.parent().expect("registration script parent");
    for module in [
        "agent_registration_support.py",
        "agent_registration_lifecycle.py",
        "agent_registration_fs.py",
    ] {
        let support = scripts.join(module);
        if !support.is_file() {
            errors.push(format!(
                "{} must exist for safe Codexy agent registration lifecycle checks",
                display_relative(&support)
            ));
        }
    }
    if !bootstrap.is_file() {
        errors.push(format!(
            "{} must exist for installed first-use specialist registration bootstrap",
            display_relative(&bootstrap)
        ));
    }
    for entrypoint in [&script, &bootstrap] {
        if entrypoint
            .symlink_metadata()
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            errors.push(format!(
                "{} must not be a symbolic link",
                display_relative(entrypoint)
            ));
        }
    }
    #[cfg(unix)]
    for entrypoint in [&script, &bootstrap] {
        if entrypoint
            .metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 == 0)
            .unwrap_or(true)
        {
            errors.push(format!(
                "{} must be executable",
                display_relative(entrypoint)
            ));
        }
    }
    errors
}
