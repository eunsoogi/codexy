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
    let command_name = first
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(first)
        .to_ascii_lowercase();
    let command_stem = command_name
        .strip_suffix(".exe")
        .or_else(|| command_name.strip_suffix(".cmd"))
        .or_else(|| command_name.strip_suffix(".bat"))
        .unwrap_or(&command_name);
    if DISALLOWED_RUNTIME_COMMANDS.contains(&command_stem) {
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::check_no_script_runtime;

    #[test]
    fn windows_suffixed_runtime_matrix_preserves_exact_diagnostics() {
        let path = Path::new(".mcp.json");

        for command in ["python3.exe", "PY.EXE", r"C:\tools\node.exe"] {
            let argv = [command.to_owned(), "--stdio".to_owned()];
            let error = check_no_script_runtime(path, "helper", &argv)
                .expect_err("Windows-suffixed script runtime unexpectedly passed")
                .to_string();
            assert_eq!(
                error,
                format!(
                    ".mcp.json helper.command must not use JS/Python runtime command {command}"
                )
            );
        }
    }
}
