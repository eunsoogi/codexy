use std::fs;

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const PACKAGE_NAME: &str = "codexy-runtime-tools";
const LAUNCHER_MARKER: &str = "# codexy-version-sync:runtime-tool";
const LAUNCHER_PREFIX: &str = "CODEXY_RUNTIME_TOOL_SPEC='codexy-runtime-tools==";
const LAUNCHER_SUFFIX: &str = "' # codexy-version-sync:runtime-tool";

fn pyproject() -> Result<std::path::PathBuf> {
    Ok(repo_root()?.join("packages/codexy-runtime-tools/pyproject.toml"))
}

fn wrappers() -> Result<Vec<std::path::PathBuf>> {
    let root = repo_root()?.join("plugins/codexy/mcp");
    Ok(["lsp", "codegraph"]
        .into_iter()
        .map(|server| root.join(format!("codexy-mcp-{server}")))
        .collect())
}

fn optional_hook_launcher() -> Result<std::path::PathBuf> {
    Ok(repo_root()?.join("plugins/codexy/hooks/codexy-admission.sh"))
}

fn launcher_marker_version(line: &str) -> Option<&str> {
    line.strip_prefix(LAUNCHER_PREFIX)?
        .strip_suffix(LAUNCHER_SUFFIX)
}

fn valid_marker_version(version: &str) -> bool {
    version == "__CODEXY_VERSION__"
        || (version.split('.').count() == 3
            && version
                .split('.')
                .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit())))
}

fn launcher_marker_line<'a>(text: &'a str, allow_placeholder: bool) -> Result<(&'a str, &'a str)> {
    if text.matches(LAUNCHER_MARKER).count() != 1 {
        bail!("optional hook launcher must contain exactly one {LAUNCHER_MARKER:?} marker");
    }
    let line = text
        .lines()
        .find(|line| line.contains(LAUNCHER_MARKER))
        .context("optional hook launcher version marker line is missing")?;
    let version = launcher_marker_version(line)
        .context("optional hook launcher version marker is malformed")?;
    if !valid_marker_version(version) || (!allow_placeholder && version == "__CODEXY_VERSION__") {
        bail!("optional hook launcher runtime-tool version is invalid: {version:?}");
    }
    Ok((line, version))
}

fn check_launcher_contents(text: Option<&str>, expected: &str) -> Result<()> {
    let Some(text) = text else {
        return Ok(());
    };
    let (_, observed) = launcher_marker_line(text, false)?;
    if observed != expected {
        bail!(
            "optional hook launcher runtime-tool version mismatch: observed={observed}, expected={expected}"
        );
    }
    Ok(())
}

fn rewrite_launcher(text: &str, requested: &str) -> Result<String> {
    let (current_line, _) = launcher_marker_line(text, true)?;
    let requested_line = format!("{LAUNCHER_PREFIX}{requested}{LAUNCHER_SUFFIX}");
    Ok(text.replacen(current_line, &requested_line, 1))
}

fn package_version(text: &str) -> Option<&str> {
    text.lines()
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
}

pub(super) fn check_version(expected: &str) -> Result<()> {
    let path = pyproject()?;
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    let observed = package_version(&text)
        .with_context(|| format!("{} must declare project version", display_relative(&path)))?;
    if observed != expected {
        bail!(
            "version mismatch: {}={observed}, plugin manifest={expected}",
            display_relative(&path)
        );
    }
    let expected_pin = format!("{PACKAGE_NAME}=={expected}");
    for wrapper in wrappers()? {
        let wrapper_text = fs::read_to_string(&wrapper)
            .with_context(|| format!("missing required file: {}", display_relative(&wrapper)))?;
        if !wrapper_text.contains(&expected_pin) {
            bail!(
                "version mismatch: {} must pin {expected_pin}",
                display_relative(&wrapper)
            );
        }
    }
    let launcher = optional_hook_launcher()?;
    let launcher_text = launcher
        .exists()
        .then(|| {
            fs::read_to_string(&launcher).with_context(|| {
                format!(
                    "reading optional hook launcher: {}",
                    display_relative(&launcher)
                )
            })
        })
        .transpose()?;
    check_launcher_contents(launcher_text.as_deref(), expected)?;
    Ok(())
}

pub(super) fn set_version(current: &str, requested: &str) -> Result<()> {
    let path = pyproject()?;
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    let current_line = format!("version = \"{current}\"");
    if text.matches(&current_line).count() != 1 {
        bail!(
            "{} must contain exactly one {current_line:?}",
            display_relative(&path)
        );
    }
    fs::write(
        &path,
        text.replace(&current_line, &format!("version = \"{requested}\"")),
    )?;
    let current_pin = format!("{PACKAGE_NAME}=={current}");
    let requested_pin = format!("{PACKAGE_NAME}=={requested}");
    for wrapper in wrappers()? {
        let text = fs::read_to_string(&wrapper)?;
        if text.matches(&current_pin).count() != 1 {
            bail!(
                "{} must contain exactly one {current_pin}",
                display_relative(&wrapper)
            );
        }
        fs::write(&wrapper, text.replace(&current_pin, &requested_pin))?;
    }
    let launcher = optional_hook_launcher()?;
    if launcher.exists() {
        let text = fs::read_to_string(&launcher).with_context(|| {
            format!(
                "reading optional hook launcher: {}",
                display_relative(&launcher)
            )
        })?;
        fs::write(&launcher, rewrite_launcher(&text, requested)?).with_context(|| {
            format!(
                "writing optional hook launcher: {}",
                display_relative(&launcher)
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check_launcher_contents, rewrite_launcher};

    const VALID: &str = "#!/bin/sh\nCODEXY_RUNTIME_TOOL_SPEC='codexy-runtime-tools==1.2.1' # codexy-version-sync:runtime-tool\n";

    #[test]
    fn optional_launcher_contract_accepts_absent_and_matching_marker() {
        assert!(check_launcher_contents(None, "1.2.1").is_ok());
        assert!(check_launcher_contents(Some(VALID), "1.2.1").is_ok());
    }

    #[test]
    fn optional_launcher_contract_rejects_malformed_mismatched_and_duplicate_markers() {
        assert!(check_launcher_contents(Some(VALID), "1.3.0").is_err());
        assert!(
            check_launcher_contents(
                Some("#!/bin/sh\n# codexy-version-sync:runtime-tool\n"),
                "1.2.1"
            )
            .is_err()
        );
        assert!(check_launcher_contents(Some(&format!("{VALID}{VALID}")), "1.2.1").is_err());
    }

    #[test]
    fn optional_launcher_placeholder_rewrites_from_the_requested_version() {
        let placeholder = "#!/bin/sh\nCODEXY_RUNTIME_TOOL_SPEC='codexy-runtime-tools==__CODEXY_VERSION__' # codexy-version-sync:runtime-tool\n";
        let rewritten = rewrite_launcher(placeholder, "1.3.0").expect("rewrite marker");
        assert!(rewritten.contains("codexy-runtime-tools==1.3.0"));
        assert!(!rewritten.contains("__CODEXY_VERSION__"));
    }
}
