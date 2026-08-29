use std::{io, path::Path, process::Output};

use crate::support::FixtureCommand as Command;

pub(super) fn contextual_error(
    stage: &str,
    path: &Path,
    details: &str,
    error: io::Error,
) -> io::Error {
    let raw_os_error = error.raw_os_error();
    io::Error::new(
        error.kind(),
        format!(
            "{stage}: path={} {details} raw_os_error={raw_os_error:?}: {error}",
            path.display()
        ),
    )
}

pub(super) fn fixture_output(command: &mut Command, path: &Path, cwd: &Path) -> io::Result<Output> {
    let program = command.get_program().to_string_lossy().into_owned();
    let argv = std::iter::once(program.clone())
        .chain(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned()),
        )
        .collect::<Vec<_>>()
        .join(" ");
    command.output().map_err(|error| {
        contextual_error(
            "spawn/output fixture command",
            path,
            &format!("executable={program} cwd={} argv=[{argv}]", cwd.display()),
            error,
        )
    })
}

pub(crate) fn assert_fixture_error_context(
    path: &Path,
    cwd: &Path,
    raw_os_error: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new(path);
    command.current_dir(cwd);
    let text = fixture_output(&mut command, path, cwd)
        .expect_err("fixture error")
        .to_string();
    let (prefix, fields) = text.split_once("path=").ok_or("path field")?;
    assert_eq!(prefix, "spawn/output fixture command: ");
    let (actual_path, fields) = fields
        .split_once(" executable=")
        .ok_or("executable field")?;
    assert_eq!(actual_path, path.to_str().ok_or("fixture path")?);
    let (actual_executable, fields) = fields.split_once(" cwd=").ok_or("cwd field")?;
    let (actual_cwd, fields) = fields.split_once(" argv=[").ok_or("argv field")?;
    let (actual_argv, fields) = fields
        .split_once("] raw_os_error=")
        .ok_or("raw error field")?;
    let (actual_raw, _) = fields.split_once(": ").ok_or("error detail")?;
    assert_eq!(actual_cwd, cwd.to_str().ok_or("cwd path")?);
    assert_eq!(actual_argv, actual_executable);
    if raw_os_error {
        assert_ne!(actual_raw, "None");
    }
    Ok(())
}
