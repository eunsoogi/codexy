use std::fs;

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const PACKAGE_NAME: &str = "codexy-runtime-tools";
const LAUNCHER_MARKER: &str = "codexy-version-sync:runtime-tool";

#[derive(Clone, Copy)]
struct LauncherMarker {
    prefix: &'static str,
    suffix: &'static str,
}

const POSIX_MARKER: LauncherMarker = LauncherMarker {
    prefix: "CODEXY_RUNTIME_TOOL_SPEC='codexy-runtime-tools==",
    suffix: "' # codexy-version-sync:runtime-tool",
};
const CMD_MARKER: LauncherMarker = LauncherMarker {
    prefix: "set \"CODEXY_RUNTIME_TOOL_SPEC=codexy-runtime-tools==",
    suffix: "\" & REM codexy-version-sync:runtime-tool",
};

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

fn optional_hook_launchers() -> Result<Vec<(std::path::PathBuf, LauncherMarker)>> {
    let root = repo_root()?.join("plugins/codexy/hooks");
    Ok(vec![
        (root.join("codexy-admission.sh"), POSIX_MARKER),
        (root.join("codexy-admission.cmd"), CMD_MARKER),
    ])
}

fn launcher_marker_version<'a>(line: &'a str, marker: LauncherMarker) -> Option<&'a str> {
    line.strip_prefix(marker.prefix)?
        .strip_suffix(marker.suffix)
}

fn valid_marker_version(version: &str) -> bool {
    version == "__CODEXY_VERSION__"
        || (version.split('.').count() == 3
            && version
                .split('.')
                .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit())))
}

fn launcher_marker_line(
    text: &str,
    allow_placeholder: bool,
    marker: LauncherMarker,
) -> Result<(&str, &str)> {
    if text.matches(LAUNCHER_MARKER).count() != 1 {
        bail!("optional hook launcher must contain exactly one {LAUNCHER_MARKER:?} marker");
    }
    let line = text
        .lines()
        .find(|line| line.contains(LAUNCHER_MARKER))
        .context("optional hook launcher version marker line is missing")?;
    let version = launcher_marker_version(line, marker)
        .context("optional hook launcher version marker is malformed")?;
    if !valid_marker_version(version) || (!allow_placeholder && version == "__CODEXY_VERSION__") {
        bail!("optional hook launcher runtime-tool version is invalid: {version:?}");
    }
    Ok((line, version))
}

fn check_launcher_contents(
    text: Option<&str>,
    expected: &str,
    marker: LauncherMarker,
) -> Result<()> {
    let Some(text) = text else {
        return Ok(());
    };
    let (_, observed) = launcher_marker_line(text, false, marker)?;
    if observed != expected {
        bail!(
            "optional hook launcher runtime-tool version mismatch: observed={observed}, expected={expected}"
        );
    }
    Ok(())
}

#[cfg(test)]
fn check_launcher_set(posix: Option<&str>, cmd: Option<&str>, expected: &str) -> Result<()> {
    check_launcher_contents(posix, expected, POSIX_MARKER)?;
    check_launcher_contents(cmd, expected, CMD_MARKER)
}

fn rewrite_launcher(text: &str, requested: &str, marker: LauncherMarker) -> Result<String> {
    let (current_line, _) = launcher_marker_line(text, true, marker)?;
    let requested_line = format!("{}{requested}{}", marker.prefix, marker.suffix);
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
    for (launcher, marker) in optional_hook_launchers()? {
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
        check_launcher_contents(launcher_text.as_deref(), expected, marker)?;
    }
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
    for (launcher, marker) in optional_hook_launchers()? {
        if launcher.exists() {
            let text = fs::read_to_string(&launcher).with_context(|| {
                format!(
                    "reading optional hook launcher: {}",
                    display_relative(&launcher)
                )
            })?;
            fs::write(&launcher, rewrite_launcher(&text, requested, marker)?).with_context(
                || {
                    format!(
                        "writing optional hook launcher: {}",
                        display_relative(&launcher)
                    )
                },
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CMD_MARKER, POSIX_MARKER, check_launcher_set, rewrite_launcher};

    const POSIX: &str = "#!/bin/sh\nCODEXY_RUNTIME_TOOL_SPEC='codexy-runtime-tools==1.2.1' # codexy-version-sync:runtime-tool\n";
    const CMD: &str = "@echo off\r\nset \"CODEXY_RUNTIME_TOOL_SPEC=codexy-runtime-tools==1.2.1\" & REM codexy-version-sync:runtime-tool\r\n";

    #[test]
    fn optional_launcher_contract_accepts_absent_posix_cmd_and_both() {
        assert!(check_launcher_set(None, None, "1.2.1").is_ok());
        assert!(check_launcher_set(Some(POSIX), None, "1.2.1").is_ok());
        assert!(check_launcher_set(None, Some(CMD), "1.2.1").is_ok());
        assert!(check_launcher_set(Some(POSIX), Some(CMD), "1.2.1").is_ok());
    }

    #[test]
    fn optional_launcher_contract_rejects_pin_mismatch_and_duplicate_markers() {
        assert!(check_launcher_set(Some(POSIX), None, "1.3.0").is_err());
        assert!(check_launcher_set(None, Some(CMD), "1.3.0").is_err());
        assert!(
            check_launcher_set(
                Some("#!/bin/sh\n# codexy-version-sync:runtime-tool\n"),
                None,
                "1.2.1"
            )
            .is_err()
        );
        assert!(check_launcher_set(Some(&format!("{POSIX}{POSIX}")), None, "1.2.1").is_err());
        assert!(check_launcher_set(None, Some(&format!("{CMD}{CMD}")), "1.2.1").is_err());
    }

    #[test]
    fn optional_launcher_placeholders_rewrite_from_the_same_requested_version() {
        let posix = POSIX.replace("1.2.1", "__CODEXY_VERSION__");
        let cmd = CMD.replace("1.2.1", "__CODEXY_VERSION__");
        for (text, marker) in [(posix, POSIX_MARKER), (cmd, CMD_MARKER)] {
            let rewritten = rewrite_launcher(&text, "1.3.0", marker).expect("rewrite marker");
            assert!(rewritten.contains("codexy-runtime-tools==1.3.0"));
            assert!(!rewritten.contains("__CODEXY_VERSION__"));
        }
    }
}
