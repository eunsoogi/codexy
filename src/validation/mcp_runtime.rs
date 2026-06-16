use std::path::Path;

use anyhow::{Result, bail};

use crate::paths::display_relative;

const DISALLOWED_RUNTIME_COMMANDS: &[&str] =
    &["node", "nodejs", "python", "python2", "python3", "py"];
const DISALLOWED_RUNTIME_SUFFIXES: &[&str] = &[
    ".js", ".mjs", ".cjs", ".jsx", ".ts", ".tsx", ".mts", ".cts", ".py", ".pyi",
];

pub(super) fn check_no_script_runtime(path: &Path, name: &str, command: &[String]) -> Result<()> {
    let Some(first) = command.first() else {
        return Ok(());
    };
    let command_name = Path::new(first)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or(first)
        .to_ascii_lowercase();
    if DISALLOWED_RUNTIME_COMMANDS.contains(&command_name.as_str()) {
        bail!(
            "{} {name}.command must not use JS/Python runtime command {first}",
            display_relative(path)
        );
    }
    for item in command {
        let lowered = item.to_ascii_lowercase();
        if DISALLOWED_RUNTIME_SUFFIXES
            .iter()
            .any(|suffix| lowered.ends_with(suffix))
        {
            bail!(
                "{} {name}.command must not reference JS/Python runtime entrypoint {item}",
                display_relative(path)
            );
        }
    }
    Ok(())
}
