use std::process::Command;

/// A test-only command factory that preserves direct native execution and launches POSIX
/// fixture scripts through `sh` on Windows.
#[derive(Debug)]
pub(crate) struct FixtureCommand(Command);

impl FixtureCommand {
    pub(crate) fn new(program: impl AsRef<std::ffi::OsStr>) -> Self {
        let program = program.as_ref();
        #[cfg(windows)]
        {
            if let Ok(contents) = std::fs::read(std::path::Path::new(program)) {
                match fixture_script_launcher(true, &contents) {
                    Ok(Some(interpreter)) => {
                        let interpreter = discover_windows_interpreter(interpreter)
                            .unwrap_or_else(|error| panic!("{error}"));
                        let mut command = Command::new(interpreter);
                        command.arg(program);
                        return Self(command);
                    }
                    Ok(None) => {}
                    Err(error) => panic!("{error}"),
                }
            }
        }
        Self(Command::new(program))
    }
}

impl std::ops::Deref for FixtureCommand {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for FixtureCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Command> for FixtureCommand {
    fn from(command: Command) -> Self {
        Self(command)
    }
}

fn fixture_script_launcher(
    is_windows: bool,
    contents: &[u8],
) -> Result<Option<&'static str>, String> {
    if !is_windows {
        return Ok(None);
    }
    let first_line = contents
        .splitn(2, |byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    let first_line = first_line.strip_suffix(b"\r").unwrap_or(first_line);
    if !first_line.starts_with(b"#!") {
        return Ok(None);
    }
    let first_line = std::str::from_utf8(first_line)
        .map_err(|_| "malformed fixture script shebang".to_owned())?;
    match first_line {
        "#!/bin/sh" => Ok(Some("sh")),
        "#!/usr/bin/env bash" => Ok(Some("bash")),
        "#!/usr/bin/env python3" => Ok(Some("python")),
        "#!" => Err("malformed fixture script shebang".to_owned()),
        _ => Err(format!("unsupported fixture script shebang: {first_line}")),
    }
}

#[cfg(windows)]
fn discover_windows_interpreter(interpreter: &str) -> Result<std::path::PathBuf, String> {
    let path = std::env::var_os("PATH").ok_or_else(|| {
        format!("Windows fixture interpreter `{interpreter}` cannot discover PATH")
    })?;
    let extensions = std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
    let extensions = extensions.to_string_lossy();
    let candidates = std::iter::once(interpreter.to_owned()).chain(
        extensions
            .split(';')
            .filter(|extension| !extension.is_empty())
            .map(|extension| format!("{interpreter}{extension}")),
    );
    for directory in std::env::split_paths(&path) {
        for candidate in candidates.clone() {
            let candidate = directory.join(candidate);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(format!(
        "Windows fixture interpreter `{interpreter}` was not found on the host PATH"
    ))
}

#[test]
fn fixture_script_launcher_maps_only_the_inventoryed_windows_shebangs() {
    assert_eq!(
        fixture_script_launcher(true, b"#!/bin/sh\necho fixture\n"),
        Ok(Some("sh"))
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/bin/sh\r\necho fixture\r\n"),
        Ok(Some("sh"))
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/usr/bin/env bash\necho fixture\n"),
        Ok(Some("bash"))
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/usr/bin/env python3\nprint('fixture')\n"),
        Ok(Some("python"))
    );
    assert_eq!(fixture_script_launcher(true, b"MZ\x90\0"), Ok(None));
    assert_eq!(
        fixture_script_launcher(false, b"#!/bin/sh\necho fixture\n"),
        Ok(None)
    );
}

#[test]
fn fixture_script_launcher_rejects_unknown_and_malformed_windows_shebangs() {
    assert_eq!(
        fixture_script_launcher(true, b"#!/usr/bin/env ruby\nputs 'fixture'\n"),
        Err("unsupported fixture script shebang: #!/usr/bin/env ruby".to_owned())
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/bin/sh-invalid\necho fixture\n"),
        Err("unsupported fixture script shebang: #!/bin/sh-invalid".to_owned())
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!\nfixture\n"),
        Err("malformed fixture script shebang".to_owned())
    );
}

#[cfg(unix)]
#[test]
fn fixture_command_preserves_script_arguments_cwd_stdin_and_exit_status()
-> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write as _;

    let temp = tempfile::tempdir()?;
    let working_directory = temp.path().join("working directory");
    std::fs::create_dir(&working_directory)?;
    let script = working_directory.join("fixture script");
    std::fs::write(
        &script,
        "#!/bin/sh\ninput=$(cat)\nprintf '%s\\n%s\\n%s\\n' \"$PWD\" \"$1\" \"$input\"\nexit 17\n",
    )?;
    super::make_executable(&script)?;

    let mut child = FixtureCommand::new(&script)
        .arg("argument with spaces")
        .current_dir(&working_directory)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .ok_or("fixture stdin")?
        .write_all(b"stdin with spaces")?;
    let output = child.wait_with_output()?;

    assert_eq!(output.status.code(), Some(17));
    assert_eq!(
        String::from_utf8(output.stdout)?,
        format!(
            "{}\nargument with spaces\nstdin with spaces\n",
            working_directory.canonicalize()?.display()
        )
    );
    Ok(())
}
