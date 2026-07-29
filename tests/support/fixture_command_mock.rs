use std::{fs, io, path::Path};

/// Writes a POSIX command mock that `sh` can find on Unix and Windows.
///
/// Windows command lookup uses PATHEXT, so the payload is paired with a `.cmd`
/// launcher instead of leaving an extensionless shell script to fall through to
/// a host executable.
pub(crate) fn write_posix_fixture_command(path: &Path, source: &str) -> io::Result<()> {
    let source = traced_source(path, source)?;
    #[cfg(windows)]
    {
        let payload = path.with_extension("sh");
        fs::write(&payload, source)?;
        super::make_executable(&payload)?;
        let shell = super::fixture_command_windows::discover_windows_interpreter("sh")
            .map_err(io::Error::other)?;
        fs::write(
            path.with_extension("cmd"),
            format!(
                "@echo off\r\n\"{}\" \"%~dp0{}.sh\" %*\r\n",
                command_path(&shell),
                path.file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "fixture command name")
                    })?
            ),
        )
    }
    #[cfg(not(windows))]
    {
        fs::write(path, source)?;
        super::make_executable(path)
    }
}

fn traced_source(path: &Path, source: &str) -> io::Result<String> {
    let body = source.strip_prefix("#!/bin/sh\n").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture command must use /bin/sh",
        )
    })?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "fixture command name"))?;
    Ok(format!(
        "#!/bin/sh\nif test -n \"${{CODEXY_FIXTURE_COMMAND_TRACE:-}}\"; then printf '%s\\n' '{name}' >> \"$CODEXY_FIXTURE_COMMAND_TRACE\"; fi\n{body}"
    ))
}

#[cfg(windows)]
fn command_path(path: &Path) -> String {
    path.to_string_lossy().replace('%', "%%")
}
