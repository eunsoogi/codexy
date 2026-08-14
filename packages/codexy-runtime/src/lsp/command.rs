use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::lsp::pathing::resolve_root;

#[derive(Debug, Clone)]
pub(crate) struct ResolvedCommand {
    // Keep launch authority native. Diagnostics may render this path, but must
    // never reconstruct a command from that lossy display value.
    pub(crate) executable: PathBuf,
    pub(crate) arguments: Vec<OsString>,
}

impl ResolvedCommand {
    pub(crate) fn display_executable(&self) -> String {
        self.executable.to_string_lossy().into_owned()
    }
}

pub(crate) fn resolve_command(command: &[String], root: Option<&str>) -> Result<Vec<String>> {
    let Some(first) = command.first() else {
        return Ok(Vec::new());
    };
    if matches!(
        command_path_kind(first, cfg!(windows)),
        CommandPathKind::Relative
    ) {
        if let Some(root) = root {
            let mut output = vec![resolve_root(root)?.join(first).display().to_string()];
            output.extend(command.iter().skip(1).cloned());
            return Ok(output);
        }
    }
    Ok(command.to_vec())
}

pub(crate) fn resolve_executable(command: &[String]) -> Result<ResolvedCommand, String> {
    let Some(executable) = command.first() else {
        return Err("server command is missing".to_owned());
    };
    if !matches!(
        command_path_kind(executable, cfg!(windows)),
        CommandPathKind::Bare
    ) {
        let path = absolute_path(PathBuf::from(executable))?;
        if is_executable(&path) {
            return Ok(ResolvedCommand {
                executable: path,
                arguments: command[1..].iter().map(OsString::from).collect(),
            });
        }
        let reason = if path.exists() {
            format!("executable is not executable: {executable}")
        } else {
            format!("executable not found: {executable}")
        };
        return Err(reason);
    }
    let executable_names = executable_names(executable);
    // Lookup can be scoped without starving the MCP process of platform loader paths.
    let search_path =
        std::env::var_os("CODEXY_LSP_LOOKUP_PATH").or_else(|| std::env::var_os("PATH"));
    for entry in search_path
        .as_deref()
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
    {
        let entry = absolute_search_entry(entry)?;
        for name in &executable_names {
            let candidate = entry.join(name);
            if is_executable(&candidate) {
                return Ok(ResolvedCommand {
                    executable: candidate,
                    arguments: command[1..].iter().map(OsString::from).collect(),
                });
            }
        }
    }
    Err(format!("executable not found on PATH: {executable}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandPathKind {
    Bare,
    Relative,
    Absolute,
}

fn command_path_kind(command: &str, is_windows: bool) -> CommandPathKind {
    if !command.contains('/') && !(is_windows && command.contains('\\')) {
        return CommandPathKind::Bare;
    }
    if Path::new(command).is_absolute() || (is_windows && windows_drive_path_is_absolute(command)) {
        CommandPathKind::Absolute
    } else {
        CommandPathKind::Relative
    }
}

fn windows_drive_path_is_absolute(command: &str) -> bool {
    let bytes = command.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

fn absolute_search_entry(entry: PathBuf) -> Result<PathBuf, String> {
    absolute_path(entry)
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path);
    }
    std::env::current_dir()
        .map(|directory| directory.join(path))
        .map_err(|error| format!("read current directory for executable lookup: {error}"))
}

fn executable_names(executable: &str) -> Vec<String> {
    executable_names_for_platform(
        executable,
        cfg!(windows),
        std::env::var_os("PATHEXT").as_deref(),
    )
}

fn executable_names_for_platform(
    executable: &str,
    is_windows: bool,
    _pathext: Option<&OsStr>,
) -> Vec<String> {
    let mut names = vec![executable.to_owned()];
    if Path::new(executable).extension().is_some() {
        return names;
    }
    if is_windows {
        names.push(format!("{executable}.exe"));
    }
    names
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    path.metadata()
        .is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;

    use super::{
        CommandPathKind, ResolvedCommand, command_path_kind, executable_names_for_platform,
    };

    #[test]
    fn windows_names_ignore_unlaunchable_pathext_shims() {
        let names = executable_names_for_platform(
            "rust-analyzer",
            true,
            Some(OsStr::new(".CMD;.BAT;.EXE")),
        );

        assert_eq!(names, vec!["rust-analyzer", "rust-analyzer.exe"]);
    }

    #[test]
    fn resolved_commands_keep_native_launch_values() {
        let executable = PathBuf::from("native-\u{c2e4}\u{d589}\u{d30c}\u{c77c}");
        let argument = OsString::from("two words");
        let command = ResolvedCommand {
            executable: executable.clone(),
            arguments: vec![argument.clone()],
        };

        assert_eq!(command.executable.as_os_str(), executable.as_os_str());
        assert_eq!(command.arguments, [argument]);
    }

    #[test]
    fn command_paths_accept_slash_forms_on_each_platform() {
        assert_eq!(
            command_path_kind(r"rust\analyzer", false),
            CommandPathKind::Bare
        );
        assert_eq!(
            command_path_kind("servers/rust-analyzer", false),
            CommandPathKind::Relative
        );
        assert_eq!(
            command_path_kind("C:/tools/rust-analyzer.exe", true),
            CommandPathKind::Absolute
        );
        assert_eq!(
            command_path_kind(r"C:\tools\rust-analyzer.exe", true),
            CommandPathKind::Absolute
        );
    }
}
