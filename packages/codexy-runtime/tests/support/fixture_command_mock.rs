use std::{fs, io, path::Path};

/// Writes a POSIX command mock that a nested `sh` resolves by its bare name.
///
/// The Windows fixtures run the production shell text through Git Bash, whose
/// POSIX lookup requires the extensionless payload rather than a PATHEXT-only
/// `.cmd` companion.
pub(crate) fn write_posix_fixture_command(path: &Path, source: &str) -> io::Result<()> {
    let source = traced_source(path, source)?;
    #[cfg(windows)]
    {
        fs::write(path, source)?;
        super::make_executable(path)
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
